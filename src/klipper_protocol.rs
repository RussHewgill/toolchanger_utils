use std::{collections::HashMap, sync::atomic::AtomicU32};

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use serde_json::Value;
use url::Url;

use crate::ui::ui_types::Axis;

#[derive(Clone)]
pub struct KlipperProtocol {
    // pub url: String,
    url: Url,
    client: reqwest::blocking::Client,
    id: std::sync::Arc<AtomicU32>,

    position: Option<(f64, f64, f64)>,
    // camera_pos: Option<(f64, f64)>,
    position_stale: bool,

    z_height: f64,
}

impl KlipperProtocol {
    pub fn new(
        // url: &str,
        url: url::Url,
    ) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .build()
            .context("Failed to build HTTP client")?;

        Ok(KlipperProtocol {
            // url: url.to_string(),
            url,
            client,
            id: std::sync::Arc::new(AtomicU32::new(1)),

            position: None,
            // camera_pos: None,
            position_stale: true,

            // z_height: 30.,
            z_height: 33.51,
        })
    }

    pub fn get_position(&mut self) -> Result<Option<(f64, f64, f64)>> {
        if self.position_stale {
            let pos = self.fetch_position()?;
            self.position_stale = false;
            Ok(Some(pos))
        } else {
            Ok(self.position)
        }
    }

    pub fn fetch_position(&mut self) -> Result<(f64, f64, f64)> {
        // let res = self.send_request("/printer/objects/query", "printer.object.toolhead")?;

        let map = serde_json::json!({
            "objects": {
                "toolhead": ["position"]
            }
        });

        // let url = format!("{}/printer/objects/query", self.url);
        let url = self.url.join("/printer/objects/query")?;

        // debug!("Sending request to {}", url);

        let res = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&map)
            .send()
            .context("Failed to send request")?;

        let json = res.json::<Value>().context("Failed to parse response")?;

        let pos = &json["result"]["status"]["toolhead"]["position"];

        let pos = pos
            .as_array()
            .ok_or_else(|| anyhow!("Failed to parse position"))?;

        let x = pos
            .get(0)
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow!("Failed to parse X position"))?;

        let y = pos
            .get(1)
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow!("Failed to parse Y position"))?;

        let z = pos
            .get(2)
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow!("Failed to parse Z position"))?;

        // debug!("Position: {}", pos);

        self.position = Some((x, y, z));

        Ok((x, y, z))
    }

    pub fn move_to_position(&mut self, pos: (f64, f64), bounce: Option<f64>) -> Result<()> {
        let z_gcode = format!("G1 Z{:.2}", self.z_height);
        self.run_gcode(&z_gcode)?;

        let x = pos.0;
        let y = pos.1;

        debug!("Moving to {} {}", x, y);

        if let Some(bounce_amount) = bounce {
            // let bounce_amount = 5.;

            let Ok((x0, y0, _)) = self.fetch_position() else {
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
            self.run_gcode(&gcode)?;
        }

        let gcode = format!("G1 X{} Y{}", x, y);
        // debug!("Running gcode 1: {}", gcode);
        self.run_gcode(&gcode)?;

        Ok(())
    }

    // #[cfg(feature = "nope")]
    pub fn move_axis_relative(
        &mut self,
        axis: Axis,
        amount: f64,
        bounce: Option<f64>,
    ) -> Result<()> {
        let axis = match axis {
            Axis::X => "X",
            Axis::Y => "Y",
            // Axis::Z => "Z",
            _ => bail!("Invalid axis"),
        };

        debug!("Moving axis {} by {}", axis, amount);

        if let Some(bounce_amount) = bounce {
            let Some((x0, y0, _)) = self.get_position()? else {
                bail!("Failed to get position");
            };

            // let bounce_amount = 5.;

            let (m0, m1) = if amount > 0.0 {
                (amount + bounce_amount, -bounce_amount)
            } else {
                (amount - bounce_amount, bounce_amount)
            };

            // debug!("Moving axis {} by {} and {}", axis, m0, m1);

            self.run_gcode(&format!("_CLIENT_LINEAR_MOVE {}={}", axis, m0))?;
            self.run_gcode(&format!("_CLIENT_LINEAR_MOVE {}={}", axis, m1))?;
        } else {
            self.run_gcode(&format!("_CLIENT_LINEAR_MOVE {}={}", axis, amount))?;
        }

        Ok(())
    }

    pub fn home_xy(&mut self) -> Result<()> {
        self.run_gcode("G28 X Y")?;
        Ok(())
    }

    pub fn home_all(&mut self) -> Result<()> {
        self.run_gcode("G28")?;
        Ok(())
    }

    pub fn pick_tool(&mut self, tool: usize) -> Result<()> {
        let gcode = format!("T{}", tool);
        self.run_gcode(&gcode)?;
        Ok(())
    }

    pub fn dropoff_tool(&mut self) -> Result<()> {
        let gcode = "T_1";
        self.run_gcode(&gcode)?;
        Ok(())
    }

    pub fn get_tool_offsets(&self) -> Result<Vec<(f64, f64, f64)>> {
        let vars = self.get_variables()?;

        let vars = &vars["result"]["status"]["save_variables"]["variables"];

        let mut offsets = Vec::new();

        for t in 0..4 {
            let x = vars[&format!("t{}_x_offset", t)]
                .as_f64()
                .ok_or_else(|| anyhow!("Failed to parse tool {} x offset", t))?;
            let y = vars[&format!("t{}_y_offset", t)]
                .as_f64()
                .ok_or_else(|| anyhow!("Failed to parse tool {} y offset", t))?;
            let z = vars[&format!("t{}_z_offset", t)]
                .as_f64()
                .ok_or_else(|| anyhow!("Failed to parse tool {} z offset", t))?;
            offsets.push((x, y, z));
        }

        Ok(offsets)
    }

    pub fn adjust_tool_offset(&mut self, tool: usize, axis: usize, amount: f64) -> Result<()> {
        let axis = match axis {
            0 => "X",
            1 => "Y",
            // 2 => "Z",
            _ => bail!("Invalid axis"),
        };

        let gcode = format!(
            "TC_ADJUST_OFFSET TOOL={} AXIS={} AMOUNT={}",
            tool, axis, amount
        );

        self.run_gcode(&gcode)?;

        Ok(())
    }

    fn get_variables(&self) -> Result<Value> {
        // let url = format!("{}/printer/objects/query", self.url);
        let url = self.url.join("/printer/objects/query")?;

        let map = serde_json::json!({
            "objects": {
                "save_variables": null
            }
        });

        let res = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&map)
            .send()
            .context("Failed to send request")?;

        let json = res.json::<Value>().context("Failed to parse response")?;

        Ok(json)
    }

    // fn run_gcode(&mut self, gcode: &str) -> Result<()> {
    pub fn run_gcode(&mut self, gcode: &str) -> Result<()> {
        /// ensure that all moves are absolute
        let gcode = format!("G90\n{}", gcode);

        let mut map = HashMap::new();
        map.insert("script", gcode);

        // let url = format!("{}/printer/gcode/script", self.url);
        let url = self.url.join("/printer/gcode/script")?;

        let res = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&map)
            .send()
            .context("Failed to send request")?;

        let res = res.status();
        if res.is_success() {
            // if get_pos {
            //     self.fetch_position()?;
            // }
            self.position_stale = true;
            Ok(())
        } else {
            bail!("Failed to run G-code: {}", res)
        }
    }
}
