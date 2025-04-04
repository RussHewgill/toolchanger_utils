use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use super::ui_types::*;

impl App {
    pub fn home_all(&mut self) {
        let Some(klipper) = &mut self.klipper else {
            debug!("klipper is not connected");
            return;
        };

        if let Err(e) = klipper.home_all() {
            error!("Failed to home all: {}", e);
        }
        self.fetch_position();
    }

    pub fn home_xy(&mut self) {
        let Some(klipper) = &mut self.klipper else {
            debug!("klipper is not connected");
            return;
        };

        if let Err(e) = klipper.home_xy() {
            error!("Failed to home XY: {}", e);
        }
        self.fetch_position();
    }

    pub fn get_position(&mut self) -> Option<(f64, f64, f64)> {
        let Some(klipper) = &mut self.klipper else {
            debug!("klipper is not connected");
            return None;
        };

        klipper.get_position().unwrap()
    }

    pub fn fetch_position(&mut self) -> Option<(f64, f64, f64)> {
        let Some(klipper) = &mut self.klipper else {
            debug!("klipper is not connected");
            return None;
        };

        // if let Some((x, y, z)) = klipper.fetch_position() {
        //     klipper.position = Some((x, y));
        //     Some((x, y, z))
        // } else {
        //     None
        // }

        klipper.fetch_position().ok()
    }

    pub fn move_to_position(&mut self, pos: (f64, f64), bounce: bool) {
        let Some(klipper) = &mut self.klipper else {
            debug!("klipper is not connected");
            return;
        };

        if let Err(e) = klipper.move_to_position(
            pos,
            if bounce {
                Some(self.options.bounce_amount)
            } else {
                None
            },
        ) {
            error!("Failed to move to position: {}", e);
        }
        // self.fetch_position();
    }

    pub fn move_relative(&mut self, amount: (f64, f64), bounce: bool) {
        let Some(klipper) = &mut self.klipper else {
            debug!("klipper is not connected");
            return;
        };

        let Ok((pos_x, pos_y, _)) = klipper.fetch_position() else {
            debug!("Failed to fetch position");
            return;
        };

        let target_x = pos_x + amount.0;
        let target_y = pos_y + amount.1;

        if let Err(e) = klipper.move_to_position(
            (target_x, target_y),
            if bounce {
                Some(self.options.bounce_amount)
            } else {
                None
            },
        ) {
            error!("Failed to move to position: {}", e);
        }

        // self.fetch_position();
    }

    pub fn move_axis_relative(&mut self, axis: Axis, amount: f64, bounce: bool) {
        let Some(klipper) = &mut self.klipper else {
            debug!("klipper is not connected");
            return;
        };

        if let Err(e) = klipper.move_axis_relative(
            axis,
            amount,
            if bounce {
                Some(self.options.bounce_amount)
            } else {
                None
            },
        ) {
            error!("Failed to move axis: {}", e);
        }
        // self.fetch_position();
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
        // self.fetch_position();
    }

    pub fn pickup_tool(&mut self, tool: i32, move_to_camera: bool) {
        if tool < 0 {
            error!("Invalid tool number: {}", tool);
            return;
        }

        let Some(klipper) = &mut self.klipper else {
            debug!("klipper is not connected");
            return;
        };

        if let Err(e) = klipper.pick_tool(tool as usize) {
            error!("Failed to pick up tool: {}", e);
        } else {
            self.active_tool = Some(tool as usize);
        }

        if move_to_camera {
            let Some((camera_x, camera_y)) = self.camera_pos else {
                error!("Camera position not set");
                return;
            };

            if let Err(e) =
                klipper.move_to_position((camera_x, camera_y), Some(self.options.bounce_amount))
            {
                error!("Failed to move to camera position: {}", e);
            }
        }
        // self.fetch_position();
    }

    pub fn adjust_offset_from_camera(&mut self, tool: usize, (x, y): (f64, f64)) {
        self.fetch_position();

        let Some((camera_x, camera_y)) = self.camera_pos else {
            error!("Camera position not set");
            return;
        };

        // let tool = self.auto_offset.as_ref().unwrap().current_tool() as usize;
        // assert_eq!(Some(tool), self.active_tool);
        let Some(tool) = self.active_tool else {
            error!("No active tool");
            return;
        };

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
        if let Err(e) = klipper.move_to_position(cam_pos, Some(self.options.bounce_amount)) {
            self.errors.push(format!("Failed to move camera: {}", e));
        }
    }
}
