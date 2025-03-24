use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use super::ui_types::*;

impl App {
    pub fn get_position(&mut self) -> Option<(f64, f64, f64)> {
        let Some(klipper) = &mut self.klipper else {
            debug!("klipper is not connected");
            return None;
        };

        klipper.get_position()
    }

    pub fn move_relative(&mut self, amount: (f64, f64), bounce: bool) {
        let Some(klipper) = &mut self.klipper else {
            debug!("klipper is not connected");
            return;
        };

        let Ok(pos) = klipper.fetch_position() else {
            debug!("Failed to fetch position");
            return;
        };

        if bounce {
            let bounce_amount = self.options.bounce_amount;

            let pos2 = (pos.0 + amount.0, pos.1 + amount.1);

            // let pos1 = (pos.1, )
        }

        #[cfg(feature = "nope")]
        if bounce {
            let bounce_amount = 0.5;

            let (x0, x1) = if amount.0 > 0.0 {
                (-bounce_amount, bounce_amount + amount.0)
            } else {
                (-bounce_amount, bounce_amount + amount.0)
            };

            let (y0, y1) = if amount.1 > 0.0 {
                (-bounce_amount, bounce_amount + amount.1)
            } else {
                (-bounce_amount, bounce_amount + amount.1)
            };

            if let Err(e) = klipper.move_to_position((pos.0 + x0, pos.1 + y0)) {
                error!("Failed to move to position: {}", e);
            }

            if let Err(e) = klipper.move_to_position((pos.0 + x1, pos.1 + y1)) {
                error!("Failed to move to position: {}", e);
            }
        } else {
            let (x, y) = (pos.0 + amount.0, pos.1 + amount.1);

            if let Err(e) = klipper.move_to_position((x, y)) {
                error!("Failed to move to position: {}", e);
            }
        }

        //
    }

    pub fn move_axis_relative(&mut self, axis: Axis, amount: f64, bounce: bool) {
        let Some(klipper) = &mut self.klipper else {
            debug!("klipper is not connected");
            return;
        };

        if let Err(e) = klipper.move_axis_relative(axis, amount, bounce) {
            error!("Failed to move axis: {}", e);
        }
    }

    pub fn dropoff_tool(&mut self) {
        let Some(klipper) = &mut self.klipper else {
            debug!("klipper is not connected");
            return;
        };

        if let Err(e) = klipper.dropoff_tool() {
            error!("Failed to drop off tool: {}", e);
        } else {
            self.active_tool = None;
        }
    }

    pub fn pickup_tool(&mut self, tool: usize) {
        let Some(klipper) = &mut self.klipper else {
            debug!("klipper is not connected");
            return;
        };

        if let Err(e) = klipper.pick_tool(tool) {
            error!("Failed to pick up tool: {}", e);
        } else {
            self.active_tool = Some(tool);
        }
    }

    pub fn adjust_offset_from_camera(&mut self, tool: usize, (x, y): (f64, f64)) {
        let Some((camera_x, camera_y)) = self.camera_pos else {
            error!("Camera position not set");
            return;
        };

        let tool = self.auto_offset.as_ref().unwrap().current_tool() as usize;

        assert_eq!(Some(tool), self.active_tool);

        let (offset_x, offset_y, _) = self.tool_offsets[tool];

        let x = x - camera_x - offset_x;
        let y = y - camera_y - offset_y;

        debug!("Applying offsets from camera: ({:.3}, {:.3})", x, y);

        let Some(klipper) = &mut self.klipper else {
            debug!("Klipper is not connected");
            return;
        };

        if let Err(e) = klipper.adjust_tool_offset(tool, 0, x) {
            self.errors
                .push(format!("Failed to adjust tool {} offset: {}", tool, e));
        } else {
            self.tool_offsets[tool].0 += x;
        }

        if let Err(e) = klipper.adjust_tool_offset(tool, 1, y) {
            self.errors
                .push(format!("Failed to adjust tool {} offset: {}", tool, e));
        } else {
            self.tool_offsets[tool].1 += y;
        }

        let cam_pos = self.camera_pos.unwrap();
        if let Err(e) = klipper.move_to_position(cam_pos) {
            self.errors.push(format!("Failed to move camera: {}", e));
        }
    }
}
