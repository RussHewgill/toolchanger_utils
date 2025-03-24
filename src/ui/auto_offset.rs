use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::time::Instant;

use crate::{klipper_protocol::KlipperProtocol, webcam::Webcam};

use super::ui_types::{App, Axis};

#[derive(Debug, Clone)]
pub struct AutoOffset {
    pub prev_position: (f64, f64),
    last_move: Instant,
    single_tool: bool,
    current_tool: i32,
}

impl AutoOffset {
    pub fn new(pos: (f64, f64), single_tool: bool) -> Self {
        AutoOffset {
            prev_position: pos,
            last_move: Instant::now(),
            single_tool,
            current_tool: -1,
        }
    }

    pub fn current_tool(&self) -> i32 {
        self.current_tool
    }
}

impl AutoOffset {
    pub const TARGET_MAX_OFFSET: f64 = 0.05;
    pub const MAX_MARGIN_OF_ERROR: f64 = 0.2;
    pub const MIN_INTERVAL_BETWEEN_MOVES: f64 = 1.0;
}

impl App {
    /// while active:
    /// wait until the error is low enough
    /// if the nozzle is in frame, but not centered, send a move command to center it
    /// re-check until it is centered with a very low error
    pub fn auto_offset(
        &mut self,
        ui: &mut egui::Ui,
        mut auto_offset: AutoOffset,
    ) -> Option<AutoOffset> {
        // let Some(auto_offset) = self.auto_offset.as_mut() else {
        //     return;
        // };

        // let some(klipper) = &mut self.klipper else {
        //     debug!("klipper is not connected");
        //     return none;
        // };

        /// if we are doing all tools, and it's the first run, dropoff the tool and pickup tool 0
        if !auto_offset.single_tool && auto_offset.current_tool < 0 {
            /// XXX: dropoff the tool first?
            // self.dropoff_tool();
            self.pickup_tool(0);
        }

        let pos = self.get_position().unwrap();

        if (pos.0, pos.1) != auto_offset.prev_position {
            auto_offset.prev_position = (pos.0, pos.1);
            self.running_average.clear();
        }

        let Some((median, moe)) = self.running_average.calculate_margin_of_error() else {
            // debug!("Failed to calculate margin of error");
            return Some(auto_offset);
        };

        if moe.0 > AutoOffset::MAX_MARGIN_OF_ERROR && moe.1 > AutoOffset::MAX_MARGIN_OF_ERROR {
            // debug!("Margin of error is too high: {:?}", moe);
            return Some(auto_offset);
        }

        let center = (Webcam::SIZE.0 as f64 / 2., Webcam::SIZE.1 as f64 / 2.);

        let offset_x = median.0 - center.0;
        let offset_y = median.1 - center.1;

        // convert pixels to mm
        let mut offset_x = offset_x / self.webcam_settings.pixels_per_mm;
        let mut offset_y = offset_y / self.webcam_settings.pixels_per_mm;

        // ui.label(format!("Offset in mm: ({}, {})", offset_x, offset_y));
        ui.label(format!("X Offset: {:.3}", offset_x));
        ui.label(format!("Y Offset: {:.3}", offset_y));

        if auto_offset.last_move.elapsed().as_secs_f64() < AutoOffset::MIN_INTERVAL_BETWEEN_MOVES {
            return Some(auto_offset);
        }

        // Rotate
        std::mem::swap(&mut offset_x, &mut offset_y);
        offset_x *= -1.0;

        if offset_x.abs() > AutoOffset::TARGET_MAX_OFFSET
            || offset_y.abs() > AutoOffset::TARGET_MAX_OFFSET
        {
            debug!("Moving to center: ({}, {})", offset_x, offset_y);

            /// move to center nozzle
            // if let Err(e) = klipper.move_axis_relative(Axis::X, offset_x, true) {
            //     error!("Failed to move X axis: {}", e);
            // }
            // self.move_axis_relative(Axis::X, offset_x, true);

            // if let Err(e) = klipper.move_axis_relative(Axis::Y, offset_y, true) {
            //     error!("Failed to move Y axis: {}", e);
            // }
            self.move_relative((offset_x, offset_y), true);

            auto_offset.last_move = Instant::now();
        } else {
            if auto_offset.single_tool {
                // nozzle is centered, halt auto offset
                self.auto_offset = None;
            } else {
                // if we are doing all tools:
                // apply offsets
                // then dropoff the tool and pickup the next one

                let Some((x, y, _)) = self.get_position() else {
                    error!("Failed to get position");
                    return Some(auto_offset);
                };

                if auto_offset.current_tool == 0 {
                    // save camera position
                    self.camera_pos = Some((x, y));
                } else {
                    self.adjust_offset_from_camera(auto_offset.current_tool as usize, (x, y));
                }

                if auto_offset.current_tool < self.options.num_tools as i32 {
                    auto_offset.current_tool += 1;
                    self.pickup_tool(auto_offset.current_tool as usize);
                }
            }
        }

        //
        Some(auto_offset)
    }
}

impl App {
    #[cfg(feature = "nope")]
    pub fn apply_offset_from_camera(&mut self, (x, y): (f64, f64)) {
        debug!("Applying offsets from camera: ({:.3}, {:.3})", x, y);

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
