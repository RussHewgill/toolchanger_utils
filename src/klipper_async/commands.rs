use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use super::KlipperConn;
use crate::ui::ui_types::Axis;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;

#[cfg(feature = "nope")]
impl KlipperConn {
    pub async fn oneshot(&self) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.url).await?;
        // debug!("Connected to {}", &url);

        let (mut ws_write, mut ws_read) = ws_stream.split();

        unimplemented!()
    }

    #[cfg(feature = "nope")]
    pub async fn fetch_position_blocking(&mut self) -> Result<(f64, f64, f64)> {
        debug!("fetching position");

        let id = self.get_id();

        debug!("id: {}", id);

        let json = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "printer.objects.query",
                "params": {
                    "objects": {
                        "gcode_move": null,
                    }
                },
                "id": id,
        });

        self.ws_write
            .send(tokio_tungstenite::tungstenite::Message::Text(
                json.to_string().into(),
            ))
            .await?;

        let Some(Ok(msg)) = self.ws_read.next().await else {
            bail!("Failed to get message from websocket");
        };

        debug!("fetch position msg: {}", msg.to_text().unwrap());

        Ok((0., 0., 0.))
    }
}

impl KlipperConn {
    pub async fn home_all(&mut self) -> Result<()> {
        self._run_gcode("G28").await
    }

    pub async fn home_xy(&mut self) -> Result<()> {
        self._run_gcode("G28 X Y").await
    }

    pub async fn get_position(&mut self) -> Result<(f64, f64, f64)> {
        let t0 = self.current_status.lock().await.last_position_update;

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
            debug!("get_position: waiting for position update");
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            let status = self.current_status.lock().await;
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

    #[cfg(feature = "nope")]
    pub async fn fetch_position(&mut self) -> Result<(f64, f64, f64)> {
        // Ok(pos)
        todo!()
    }

    pub async fn pick_tool(&mut self, tool: usize) -> Result<()> {
        let gcode = format!("T{}", tool);
        self._run_gcode(&gcode).await
    }

    pub async fn dropoff_tool(&mut self) -> Result<()> {
        let gcode = "T_1";
        self._run_gcode(&gcode).await
    }

    pub async fn move_to_position(
        &mut self,
        pos: (f64, f64, f64),
        bounce: Option<f64>,
    ) -> Result<()> {
        let Ok((x0, y0, _)) = self.get_position().await else {
            bail!("Failed to get position");
        };

        debug!("x0: {}, y0: {}", x0, y0);

        Ok(())
    }

    #[cfg(feature = "nope")]
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

            let Ok((x0, y0, _)) = self.fetch_position_blocking().await else {
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

    pub fn move_axis_relative(
        &mut self,
        axis: Axis,
        amount: f64,
        bounce: Option<f64>,
    ) -> Result<()> {
        unimplemented!()
    }
}

impl KlipperConn {
    async fn set_relative(&mut self) -> Result<()> {
        self._run_gcode("G91").await
    }

    async fn set_absolute(&mut self) -> Result<()> {
        self._run_gcode("G90").await
    }

    async fn run_gcode(&mut self, gcode: &str) -> Result<()> {
        // if !self.current_status.absolute_coordinates {
        //     self.set_absolute().await?;
        // }

        todo!()
    }

    async fn _run_gcode(&mut self, gcode: &str) -> Result<()> {
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
