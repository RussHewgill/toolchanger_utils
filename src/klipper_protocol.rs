use std::{collections::HashMap, sync::atomic::AtomicU32};

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use serde_json::Value;

pub struct KlipperProtocol {
    pub url: String,
    client: reqwest::blocking::Client,
    id: AtomicU32,

    position: Option<(f64, f64, f64)>,
    // cxc_pos: Option<(f64, f64)>,
}

impl KlipperProtocol {
    pub fn new(url: &str) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .build()
            .context("Failed to build HTTP client")?;

        Ok(KlipperProtocol {
            url: url.to_string(),
            client,
            id: AtomicU32::new(1),

            position: None,
            // cxc_pos: None,
        })
    }

    pub fn get_position(&self) -> Option<(f64, f64, f64)> {
        self.position
    }

    pub fn fetch_position(&mut self) -> Result<(f64, f64, f64)> {
        // let res = self.send_request("/printer/objects/query", "printer.object.toolhead")?;

        let map = serde_json::json!({
            "objects": {
                "toolhead": ["position"]
            }
        });

        let url = format!("{}/printer/objects/query", self.url);

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

    pub fn move_cxc(&mut self, pos: (f64, f64)) -> Result<()> {
        let gcode = "G1 Z30";
        self.run_gcode(&gcode, false)?;

        let x = pos.0;
        let y = pos.1;

        let gcode = format!("G1 X{} Y{}", x - 0.5, y - 0.5);
        self.run_gcode(&gcode, false)?;

        let gcode = format!("G1 X{} Y{}", x, y);
        self.run_gcode(&gcode, true)?;
        Ok(())
    }

    pub fn move_axis(&mut self, axis: usize, amount: f64, bounce: bool) -> Result<()> {
        let axis = match axis {
            0 => "X",
            1 => "Y",
            2 => "Z",
            _ => bail!("Invalid axis"),
        };

        debug!("Moving axis {} by {}", axis, amount);

        if bounce {
            let bounce_amount = 0.5;

            // let (m0, m1) = if amount > 0.0 {
            //     (-bounce_amount, bounce_amount + amount)
            // } else {
            //     (bounce_amount, amount - bounce_amount)
            // };

            let (m0, m1) = if amount > 0.0 {
                (-bounce_amount, bounce_amount + amount)
            } else {
                (-bounce_amount, bounce_amount + amount)
            };

            // let m0 = -amount.signum() * bounce_amount;
            // let m1 = -amount.signum() * (amount - bounce_amount * amount.signum());

            debug!("m0 = {}", m0);
            debug!("m1 = {}", m1);

            self.run_gcode(&format!("_CLIENT_LINEAR_MOVE {}={}", axis, m0), false)?;
            self.run_gcode(&format!("_CLIENT_LINEAR_MOVE {}={}", axis, m1), true)?;
        } else {
            self.run_gcode(&format!("_CLIENT_LINEAR_MOVE {}={}", axis, amount), true)?;
        }
        Ok(())
    }

    pub fn home_xy(&mut self) -> Result<()> {
        self.run_gcode("G28 X Y", true)?;
        Ok(())
    }

    pub fn home_all(&mut self) -> Result<()> {
        self.run_gcode("G28", true)?;
        Ok(())
    }

    pub fn pick_tool(&mut self, tool: usize) -> Result<()> {
        let gcode = format!("T{}", tool);
        self.run_gcode(&gcode, true)?;
        Ok(())
    }

    pub fn dropoff_tool(&mut self) -> Result<()> {
        let gcode = "T_1";
        self.run_gcode(&gcode, true)?;
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

    fn get_variables(&self) -> Result<Value> {
        let url = format!("{}/printer/objects/query", self.url);

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
    pub fn run_gcode(&mut self, gcode: &str, get_pos: bool) -> Result<()> {
        let mut map = HashMap::new();
        map.insert("script", gcode);

        let url = format!("{}/printer/gcode/script", self.url);

        let res = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&map)
            .send()
            .context("Failed to send request")?;

        let res = res.status();
        if res.is_success() {
            if get_pos {
                self.fetch_position()?;
            }
            Ok(())
        } else {
            bail!("Failed to run G-code: {}", res)
        }
    }
}
