use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::RichText;
use egui_extras::Column;
use std::time::Instant;

use crate::vision::VisionSettings;

use super::ui_types::{App, Axis};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoOffsetType {
    None,
    SingleTool,
    AllTools,
    RepeatabilityTest,
    HomingTest,
}

impl App {
    pub fn auto_offset(&mut self, ui: &mut egui::Ui) {
        self._auto_offset_ui(ui);
    }

    fn _auto_offset_ui(&mut self, ui: &mut egui::Ui) {
        let Some(pos) = self.get_position() else {
            ui.label("No position data available");
            return;
        };

        // if (pos.0, pos.1) != self.auto_offset.prev_position {
        //     self.auto_offset.prev_position = (pos.0, pos.1);
        //     self.running_average.clear();
        // }

        if matches!(self.auto_offset.auto_offset_type, AutoOffsetType::AllTools) {
            if self.auto_offset.current_tool == -1 {
                /// reset tool offsets to 0
                for tool in 0..self.options.num_tools {
                    self.set_tool_offset(tool, Axis::X, 0.0);
                    self.set_tool_offset(tool, Axis::Y, 0.0);
                }

                self.dropoff_tool();
                self.pickup_tool(0, true);
                self.auto_offset.current_tool = 0;

                return;
            }
        }

        let mut confidence = self.running_average.confidence();
        let mut guess = self.running_average.current_guess();

        if confidence.is_none() || guess.is_none() {
            // warn!("No confidence or guess data available");
            confidence = Some((-1., (0., 0., 0.)));
            guess = Some((0., 0., 0.));
        }

        ui.horizontal(|ui| {
            if ui.button("Clear Running Average").clicked() {
                self.running_average.clear();
            }

            ui.checkbox(
                &mut self.options.auto_offset_settings.swap_axes,
                "Swap Axes",
            );
            ui.checkbox(
                &mut self.options.auto_offset_settings.mirror_axes.0,
                "Mirror X Axis",
            );
            ui.checkbox(
                &mut self.options.auto_offset_settings.mirror_axes.1,
                "Mirror Y Axis",
            );
        });

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                egui_extras::TableBuilder::new(ui)
                    .id_salt("Running Average Table")
                    .column(Column::exact(100.))
                    .columns(Column::exact(80.), 5)
                    .striped(true)
                    .header(20., |mut row| {
                        row.col(|ui| {});
                        row.col(|ui| {});
                        row.col(|ui| {
                            ui.label("X");
                        });
                        row.col(|ui| {
                            ui.label("Y");
                        });
                        row.col(|ui| {
                            ui.label("radius");
                        });
                    })
                    .body(|mut body| {
                        if let Some((confidence, (c_x, c_y, c_r))) = confidence {
                            body.row(20., |mut row| {
                                row.col(|ui| {
                                    ui.label("Confidence:");
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.3}", confidence));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.4}", c_x));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.4}", c_y));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.1}", c_r));
                                });
                                row.col(|ui| {});
                            });
                        }
                        if let Some((x, y, r)) = guess {
                            body.row(20., |mut row| {
                                let (x, y, r) = self._pixels_to_mm_from_center(x, y, r);

                                row.col(|ui| {
                                    ui.label("Current Guess:");
                                });
                                row.col(|_| {});
                                row.col(|ui| {
                                    ui.label(format!("{:.4}", x));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.4}", y));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.1}", r));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.0}", std::f64::consts::PI * r * r));
                                });
                            });
                        }
                    });
            });

            ui.separator();

            ui.vertical(|ui| {
                ui.group(|ui| {
                    self.auto_offset_controls(ui);
                    ui.allocate_space(ui.available_size());
                });
            });
        });

        if confidence.is_some() && guess.is_some() {
            let confidence = confidence.unwrap();
            let guess = guess.unwrap();

            let (confidence, (c_x, c_y, c_r)) = confidence;
            let (x, y, r) = guess;
            let (x, y, r) = self._pixels_to_mm_from_center(x, y, r);

            if confidence < self.options.auto_offset_settings.min_confidence_for_move {
                // ui.label("Confidence is too low to move");
                return;
            }

            if self.auto_offset.last_move.elapsed().as_secs_f64()
                < self.options.auto_offset_settings.min_interval_between_moves
            {
                // ui.label("Waiting for interval between moves");
                return;
            }

            let move_x = x;
            let move_y = y;

            let (move_x, move_y) = self._apply_screen_transform((move_x, move_y));

            match self.auto_offset.auto_offset_type() {
                AutoOffsetType::None => {}
                AutoOffsetType::SingleTool => self._auto_offset_single(ui, (move_x, move_y)),
                AutoOffsetType::AllTools => self._auto_offset_all(ui, (move_x, move_y)),
                AutoOffsetType::RepeatabilityTest => {
                    self._auto_offset_repeatability(ui, (move_x, move_y))
                }
                AutoOffsetType::HomingTest => {
                    self._auto_offset_repeatability(ui, (move_x, move_y));
                }
            }

            //
        }

        //
    }

    fn auto_offset_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let resp = ui.add(
                egui::Slider::new(
                    &mut self.options.auto_offset_settings.target_max_offset,
                    0.0..=0.05,
                )
                .text("Target Accuracy")
                .show_value(true),
            );

            ui.label(format!(
                "(resolution: {:.5})",
                self.klipper_status_frame
                    .as_ref()
                    .map(|f| f.resolution)
                    .unwrap_or(-1.)
            ));
        });

        ui.horizontal(|ui| {
            // let confidence = self.running_average.confidence();
            if let Some(guess) = self.running_average.current_guess() {
                if let Some(pos) = self.get_position() {
                    let (x, y, r) = self._pixels_to_mm_from_center(guess.0, guess.1, guess.2);

                    let (x, y) = self._apply_screen_transform((x, y));

                    let x = pos.0 + x;
                    let y = pos.1 + y;

                    ui.label(format!("Guessed position: {:.5}, {:.5}", x, y));
                }
            }
        });

        ui.horizontal(|ui| {
            ui.checkbox(
                &mut self.options.auto_offset_settings.park_tool,
                "Park tool/control test: ",
            );
        });
    }

    fn _auto_offset_single(&mut self, ui: &mut egui::Ui, (x, y): (f64, f64)) {
        if x.abs() < self.options.auto_offset_settings.target_max_offset
            && y.abs() < self.options.auto_offset_settings.target_max_offset
        {
            // ui.label("Offset is within target range");
            self.auto_offset.stop();
            return;
        }

        /// if the nozzle isn't centered, but the offset is too small to move, stop
        if x.abs() < self.options.auto_offset_settings.resolution
            && y.abs() < self.options.auto_offset_settings.resolution
        {
            self.auto_offset.stop();
            return;
        }

        debug!("Moving to center: ({:.4}, {:.4})", x, y);

        // self.move_axis_relative(Axis::X, x, true);
        // self.move_axis_relative(Axis::Y, y, true);

        if x.abs() > self.options.auto_offset_settings.target_max_offset
            && x.abs() > self.options.auto_offset_settings.resolution
        {
            if x.abs() < 0.01 {
                debug!("Fine tuning X axis: ({:.5})", x / 4.);
                self.move_axis_relative(Axis::X, x / 4., true);
            } else {
                self.move_axis_relative(Axis::X, x, true);
            }
        }

        if y.abs() > self.options.auto_offset_settings.target_max_offset
            && y.abs() > self.options.auto_offset_settings.resolution
        {
            if y.abs() < 0.01 {
                debug!("Fine tuning Y axis: ({:.5})", y / 4.);
                self.move_axis_relative(Axis::Y, y / 4., true);
            } else {
                self.move_axis_relative(Axis::Y, y, true);
            }
        }

        self.auto_offset.last_move = Instant::now();
    }

    /// to auto offset all tools:
    /// first, pick up each tool, measure offset, and save it
    /// repeat multiple times to get a good average
    fn _auto_offset_all(&mut self, ui: &mut egui::Ui, (x, y): (f64, f64)) {
        // debug!("_auto_offset_all");

        let mut stop = false;

        /// if the nozzle is centered, stop
        if x.abs() < self.options.auto_offset_settings.target_max_offset
            && y.abs() < self.options.auto_offset_settings.target_max_offset
        {
            stop = true;
        }

        /// if the nozzle isn't centered, but the offset is too small to move, stop
        if x.abs() < self.options.auto_offset_settings.resolution
            && y.abs() < self.options.auto_offset_settings.resolution
        {
            stop = true;
        }

        if stop {
            let Some(pos) = self.get_position() else {
                warn!("No position data available");
                return;
            };
            self.auto_offset.offsets[self.auto_offset.current_tool as usize]
                .push(((pos.0, pos.1), (x, y)));

            if self.auto_offset.offsets[self.auto_offset.current_tool as usize].len()
                >= self.options.auto_offset_settings.samples_per_tool
            {
                if self.auto_offset.current_tool == self.options.num_tools as i32 - 1 {
                    // done sampling all tools

                    self.process_offsets();

                    self.auto_offset.stop();
                } else {
                    // finished sampling this tool, move to next
                    self.auto_offset.current_tool += 1;
                    self.pickup_tool(self.auto_offset.current_tool, true);

                    self.running_average.clear();
                    self.auto_offset.last_move = Instant::now() + std::time::Duration::from_secs(1);
                }
            } else {
                /// found center, parking and unparking
                self.dropoff_tool();
                self.pickup_tool(self.auto_offset.current_tool, true);

                self.running_average.clear();
                self.auto_offset.last_move = Instant::now() + std::time::Duration::from_secs(1);
            }
            return;
        } else {
            debug!("Moving to center: ({:.4}, {:.4})", x, y);

            if x.abs() > self.options.auto_offset_settings.target_max_offset
                && x.abs() > self.options.auto_offset_settings.resolution
            {
                if x.abs() < 0.02 {
                    debug!("Fine tuning X axis: ({:.5})", x / 4.);
                    self.move_axis_relative(Axis::X, x / 4., true);
                } else {
                    self.move_axis_relative(Axis::X, x, true);
                }
            }

            if y.abs() > self.options.auto_offset_settings.target_max_offset
                && y.abs() > self.options.auto_offset_settings.resolution
            {
                if y.abs() < 0.02 {
                    debug!("Fine tuning Y axis: ({:.5})", y / 4.);
                    self.move_axis_relative(Axis::Y, y / 4., true);
                } else {
                    self.move_axis_relative(Axis::Y, y, true);
                }
            }

            self.auto_offset.last_move = Instant::now();
        }
    }

    fn _auto_offset_repeatability(&mut self, ui: &mut egui::Ui, (x, y): (f64, f64)) {
        let mut stop = false;

        debug!("Checking repeatability: ({:.4}, {:.4})", x, y);

        /// if the nozzle is centered, stop
        if x.abs() < self.options.auto_offset_settings.target_max_offset
            && y.abs() < self.options.auto_offset_settings.target_max_offset
        {
            stop = true;
        }

        /// if the nozzle isn't centered, but the offset is too small to move, stop
        if x.abs() < self.options.auto_offset_settings.resolution
            && y.abs() < self.options.auto_offset_settings.resolution
        {
            stop = true;
        }

        let Some(pos) = self.get_position() else {
            warn!("No position data available");
            return;
        };
        self.auto_offset
            .repeatability
            .push(((pos.0, pos.1), (x, y)));

        if stop {
            if self.auto_offset.check_repeatability == 0 {
                let t = self.auto_offset.auto_offset_type;
                self.auto_offset.stop();

                self.auto_offset
                    .process_repeatibility(matches!(t, AutoOffsetType::HomingTest));

                return;
            } else {
                debug!("Found center, adding to repeatability data");

                self.auto_offset.check_repeatability -= 1;

                /// save screenshot
                #[cfg(feature = "nope")]
                {
                    let now = std::time::SystemTime::now();
                    let now: chrono::DateTime<chrono::Utc> = now.into();
                    let now = now.format("%Y-%m-%d_%H-%M-%S");

                    let path_dir = format!(
                        "data_output/screenshots_T{}_{}",
                        self.auto_offset.current_tool, now
                    );

                    if !std::path::Path::new(&path_dir).exists() {
                        std::fs::create_dir(&path_dir).unwrap_or_else(|_| {
                            error!("Failed to create output directory: data_output");
                        });
                    }

                    let path = format!("{}/{:>02}.jpg", path_dir, self.auto_offset.current_n,);

                    self.auto_offset.current_n += 1;

                    self.channel_to_vision
                        .as_ref()
                        .unwrap()
                        .send(crate::vision::WebcamCommand::SaveScreenshot(
                            None,
                            Some(path),
                        ))
                        .unwrap_or_else(|_| {
                            error!("Failed to send snapshot command to vision thread");
                        });
                }

                match self.auto_offset.auto_offset_type {
                    AutoOffsetType::RepeatabilityTest => {
                        if self.options.auto_offset_settings.park_tool {
                            self.dropoff_tool();
                        } else {
                            self.move_to_position((30., 220.), true);
                        }
                        self.pickup_tool(self.auto_offset.current_tool, true);
                        self.running_average.clear();
                        self.auto_offset.last_move =
                            Instant::now() + std::time::Duration::from_secs(1);
                    }
                    AutoOffsetType::HomingTest => {
                        let Some(cam_pos) = self.camera_pos else {
                            warn!("No camera position data available");
                            self.auto_offset.stop();
                            return;
                        };
                        self.home_xy();
                        self.move_to_position(cam_pos, true);
                        self.running_average.clear();
                        self.auto_offset.last_move =
                            Instant::now() + std::time::Duration::from_secs(1);
                    }
                    _ => unreachable!(),
                }

                return;
            }
        }

        debug!("Moving to center: ({:.4}, {:.4})", x, y);

        if x.abs() > self.options.auto_offset_settings.target_max_offset
            && x.abs() > self.options.auto_offset_settings.resolution
        {
            if x.abs() < 0.02 {
                debug!("Fine tuning X axis: ({:.5})", x / 4.);
                self.move_axis_relative(Axis::X, x / 4., true);
            } else {
                self.move_axis_relative(Axis::X, x, true);
            }
        }

        if y.abs() > self.options.auto_offset_settings.target_max_offset
            && y.abs() > self.options.auto_offset_settings.resolution
        {
            if y.abs() < 0.02 {
                debug!("Fine tuning Y axis: ({:.5})", y / 4.);
                self.move_axis_relative(Axis::Y, y / 4., true);
            } else {
                self.move_axis_relative(Axis::Y, y, true);
            }
        }

        self.auto_offset.last_move = Instant::now();
    }

    fn _pixels_to_mm_from_center(&self, x: f64, y: f64, r: f64) -> (f64, f64, f64) {
        let center = (
            self.options.camera_size.0 / 2.,
            self.options.camera_size.1 / 2.,
        );

        let offset_x = center.0 - x;
        let offset_y = center.1 - y;
        // convert pixels to mm
        let x = offset_x / self.vision_settings.pixels_per_mm;
        let y = offset_y / self.vision_settings.pixels_per_mm;

        let r = r / self.vision_settings.pixels_per_mm;

        (x, y, r)
    }

    fn _apply_screen_transform(&self, (mut x, mut y): (f64, f64)) -> (f64, f64) {
        if self.options.auto_offset_settings.swap_axes {
            std::mem::swap(&mut x, &mut y);
        }
        if self.options.auto_offset_settings.mirror_axes.0 {
            // debug!("Mirroring X axis");
            x *= -1.0;
        }
        if self.options.auto_offset_settings.mirror_axes.1 {
            // debug!("Mirroring Y axis");
            y *= -1.0;
        }
        (x, y)
    }
}

#[cfg(feature = "nope")]
fn correct_distortion(dx: f64, dy: f64) -> (f64, f64) {
    // Get the radial distance from center (in pixels)
    let r_squared = dx * dx + dy * dy;

    let radial_distortion_k1 = 0.0;
    let radial_distortion_k2 = 0.0;

    // Apply radial distortion correction
    // k1, k2 are distortion coefficients that you'll need to calibrate
    let k1 = radial_distortion_k1; // Add this to VisionSettings
    let k2 = radial_distortion_k2; // Add this to VisionSettings

    let distortion_factor = 1.0 + k1 * r_squared + k2 * r_squared * r_squared;

    // Apply the correction
    let corrected_x = dx * distortion_factor;
    let corrected_y = dy * distortion_factor;

    (corrected_x, corrected_y)
}
