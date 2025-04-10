use anyhow::{anyhow, bail, ensure, Context, Result};
use tokio::time::Instant;
use tracing::{debug, error, info, trace, warn};

use super::KlipperConn;
use crate::ui::ui_types::Axis;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;

impl KlipperConn {
    pub async fn home_all(&mut self) -> Result<()> {
        self.run_gcode("G28").await
    }

    pub async fn home_xy(&mut self) -> Result<()> {
        self.run_gcode("G28 X Y").await
    }

    pub async fn get_position(&mut self) -> Result<(f64, f64, f64)> {
        let t0 = self.current_status.read().await.last_position_update;

        let json = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "printer.objects.query",
                "params": {
                    "objects": {
                        "gcode_move": null,
                    }
                },
                "id": self.get_id(),
        });

        self.ws_write
            .send(tokio_tungstenite::tungstenite::Message::Text(
                json.to_string().into(),
            ))
            .await?;

        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

        let pos = loop {
            // debug!("get_position: waiting for position update");
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            let status = self.current_status.read().await;
            let t1 = status.last_position_update;
            if t1 > t0 {
                break status.position;
            }
        };

        if let Some(pos) = pos {
            Ok(pos)
        } else {
            bail!("Failed to get position")
        }
    }

    pub async fn pick_tool(&mut self, tool: u32) -> Result<()> {
        let gcode = format!("T{}", tool);
        self.run_gcode(&gcode).await
    }

    pub async fn dropoff_tool(&mut self) -> Result<()> {
        let gcode = "T_1";
        self.run_gcode(&gcode).await
    }

    pub async fn move_to_position(
        &mut self,
        pos: (f64, f64, f64),
        bounce: Option<f64>,
    ) -> Result<()> {
        let z_gcode = format!("G1 Z{:.2}", pos.2);
        self.run_gcode(&z_gcode).await?;

        let x = pos.0;
        let y = pos.1;

        debug!("Moving to {} {}", x, y);

        if let Some(bounce_amount) = bounce {
            // let bounce_amount = 5.;

            let Ok((x0, y0, _)) = self.get_position().await else {
                bail!("Failed to get position");
            };

            let x2 = if x0 >= x {
                x - bounce_amount
            } else {
                x + bounce_amount
            };
            let y2 = if y0 >= y {
                y - bounce_amount
            } else {
                y + bounce_amount
            };

            // let gcode = format!("G1 X{} Y{}", x - bounce_amount, y - bounce_amount);

            let gcode = format!("G1 X{} Y{}", x2, y2);
            // debug!("Running gcode 0: {}", gcode);
            self.run_gcode(&gcode).await?;
        }

        let gcode = format!("G1 X{} Y{}", x, y);
        // debug!("Running gcode 1: {}", gcode);
        self.run_gcode(&gcode).await
    }

    pub async fn move_axis_relative(
        &mut self,
        axis: Axis,
        amount: f64,
        bounce: Option<f64>,
    ) -> Result<()> {
        let axis = match axis {
            Axis::X => "X",
            Axis::Y => "Y",
            Axis::Z => "Z",
            // _ => bail!("Invalid axis"),
        };

        debug!("Moving axis {} by {}", axis, amount);

        if axis == "Z" {
            self.run_gcode(&format!("_CLIENT_LINEAR_MOVE Z={} F=500", amount))
                .await?;
        } else if let Some(bounce_amount) = bounce {
            let Ok((x0, y0, _)) = self.get_position().await else {
                bail!("Failed to get position");
            };

            // let bounce_amount = 5.;

            let (m0, m1) = if amount > 0.0 {
                (amount + bounce_amount, -bounce_amount)
            } else {
                (amount - bounce_amount, bounce_amount)
            };

            // debug!("Moving axis {} by {} and {}", axis, m0, m1);

            self.run_gcode(&format!("_CLIENT_LINEAR_MOVE {}={}", axis, m0))
                .await?;
            self.run_gcode(&format!("_CLIENT_LINEAR_MOVE {}={}", axis, m1))
                .await?;
        } else {
            self.run_gcode(&format!("_CLIENT_LINEAR_MOVE {}={}", axis, amount))
                .await?;
        }

        Ok(())
    }

    pub async fn disable_motors(&mut self) -> Result<()> {
        self.run_gcode("M18").await
    }

    pub async fn wait_for_moves(&mut self) -> Result<()> {
        self.run_gcode("M400").await
    }

    pub async fn dwell(&mut self, ms: u32) -> Result<()> {
        self.run_gcode(&format!("G4 P{}", ms)).await
    }

    pub async fn get_offsets(&mut self) -> Result<()> {
        let vars = self.get_variables().await?;

        let mut offsets = Vec::new();

        let mut t = 0;
        loop {
            let Some(x) = vars[&format!("t{}_x_offset", t)].as_f64() else {
                // anyhow!("Failed to parse tool {} x offset", t);
                break;
            };
            let y = vars[&format!("t{}_y_offset", t)]
                .as_f64()
                .ok_or_else(|| anyhow!("Failed to parse tool {} y offset", t))?;
            let z = vars[&format!("t{}_z_offset", t)]
                .as_f64()
                .ok_or_else(|| anyhow!("Failed to parse tool {} z offset", t))?;
            offsets.push((x, y, z));

            t += 1;
        }

        self.inbox
            .send(super::KlipperMessage::ToolOffsets(offsets))
            .map_err(|e| anyhow!("Failed to send tool offsets: {:?}", e))?;

        Ok(())
    }
}

impl KlipperConn {
    async fn set_relative(&mut self) -> Result<()> {
        self.run_gcode("G91").await
    }

    async fn set_absolute(&mut self) -> Result<()> {
        self.run_gcode("G90").await
    }

    async fn get_variables(&mut self) -> Result<serde_json::Value> {
        let t0 = Instant::now();
        debug!("getting vars");
        let json = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "printer.objects.query",
                "params": {
                    "objects": {
                        "save_variables": null,
                    }
                },
                "id": self.get_id(),
        });

        self.ws_write
            .send(tokio_tungstenite::tungstenite::Message::Text(
                json.to_string().into(),
            ))
            .await?;

        let vars = loop {
            let status = &self.current_status.read().await;

            if let Some(vars) = status.vars.as_ref() {
                if t0 > vars.0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    continue;
                } else {
                    break vars.1.clone();
                }
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        };

        self.current_status.write().await.vars = None;

        Ok(vars)
    }

    async fn run_gcode(&mut self, gcode: &str) -> Result<()> {
        let json = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "printer.gcode.script",
                "params": {
                    "script": gcode,
                },
                "id": self.get_id(),

        });

        self.ws_write
            .send(tokio_tungstenite::tungstenite::Message::Text(
                json.to_string().into(),
            ))
            .await?;

        Ok(())
    }
}
