use anyhow::{anyhow, bail, ensure, Context, Result};
use egui::RichText;
use egui_extras::Column;
use tracing::{debug, error, info, trace, warn};

use std::time::Instant;

use crate::{klipper_protocol::KlipperProtocol, vision::VisionSettings};

use super::ui_types::{App, Axis};

#[derive(Debug, Clone)]
pub struct AutoOffset {
    auto_offset_type: AutoOffsetType,

    pub prev_position: (f64, f64),
    last_move: Instant,
    current_tool: i32,

    check_repeatability: usize,

    current_n: usize,

    /// (position, guessed offset from center)
    repeatability: Vec<((f64, f64), (f64, f64))>,
}

// #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[derive(Debug, Clone)]
pub struct AutoOffsetSettings {
    pub target_max_offset: f64,
    // pub max_margin_of_error: f64,
    pub min_confidence_for_move: f64,
    pub min_interval_between_moves: f64,
    pub swap_axes: bool,
    pub mirror_axes: (bool, bool),
    pub resolution: f64,
}

impl Default for AutoOffsetSettings {
    fn default() -> Self {
        AutoOffsetSettings {
            // target_max_offset: 0.01,
            target_max_offset: 0.00625,
            min_confidence_for_move: 0.95,
            min_interval_between_moves: 2.0,
            swap_axes: true,
            mirror_axes: (false, true),
            resolution: 0.00625,
        }
    }
}

impl Default for AutoOffset {
    fn default() -> Self {
        AutoOffset {
            auto_offset_type: AutoOffsetType::None,
            prev_position: (0.0, 0.0),
            last_move: Instant::now(),
            current_tool: -1,
            check_repeatability: 0,
            current_n: 0,
            repeatability: Vec::new(),
        }
    }
}

impl AutoOffset {
    pub fn auto_offset_type(&self) -> AutoOffsetType {
        self.auto_offset_type
    }

    pub fn stop(&mut self) {
        // *self = Self::default();
        self.auto_offset_type = AutoOffsetType::None;
    }

    pub fn start_single(&mut self, pos: (f64, f64), tool: i32) {
        *self = Self::default();

        self.auto_offset_type = AutoOffsetType::SingleTool;
        self.prev_position = pos;
        self.current_tool = tool;
    }

    pub fn start_all_tools(&mut self, pos: (f64, f64)) {
        unimplemented!()
    }

    pub fn start_repeatability(&mut self, pos: (f64, f64), tool: i32) {
        *self = Self::default();

        self.auto_offset_type = AutoOffsetType::RepeatabilityTest;
        self.prev_position = pos;
        self.current_tool = tool;
        self.check_repeatability = 10;
    }

    pub fn start_homing(&mut self, pos: (f64, f64), tool: i32) {
        *self = Self::default();

        self.auto_offset_type = AutoOffsetType::HomingTest;
        self.prev_position = pos;
        self.current_tool = tool;
        self.check_repeatability = 10;
    }

    pub fn process_repeatibility(&self, homing: bool) {
        debug!("Repeatability results:");

        let mut xs = self
            .repeatability
            .iter()
            .map(|((x, _), _)| *x)
            .collect::<Vec<_>>();
        let mut ys = self
            .repeatability
            .iter()
            .map(|((_, y), _)| *y)
            .collect::<Vec<_>>();

        /// calculate median:
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        ys.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let median_x = xs[xs.len() / 2];
        let median_y = ys[ys.len() / 2];

        debug!("Median: ({:.3}, {:.3})", median_x, median_y);

        // Calculate mean
        let mean_x = xs.iter().sum::<f64>() / xs.len() as f64;
        let mean_y = ys.iter().sum::<f64>() / ys.len() as f64;

        // Calculate standard deviation
        let variance_x = xs.iter().map(|&x| (x - mean_x).powi(2)).sum::<f64>() / xs.len() as f64;
        let variance_y = ys.iter().map(|&y| (y - mean_y).powi(2)).sum::<f64>() / ys.len() as f64;
        let std_dev_x = variance_x.sqrt();
        let std_dev_y = variance_y.sqrt();

        // Calculate min, max and range
        let min_x = *xs.first().unwrap();
        let max_x = *xs.last().unwrap();
        let min_y = *ys.first().unwrap();
        let max_y = *ys.last().unwrap();
        let range_x = max_x - min_x;
        let range_y = max_y - min_y;

        debug!("Median: ({:.3}, {:.3})", median_x, median_y);
        debug!("Mean: ({:.3}, {:.3})", mean_x, mean_y);
        debug!("Standard Deviation: ({:.3}, {:.3})", std_dev_x, std_dev_y);
        debug!("Min: ({:.3}, {:.3})", min_x, min_y);
        debug!("Max: ({:.3}, {:.3})", max_x, max_y);
        debug!("Range: ({:.3}, {:.3})", range_x, range_y);

        let output_dir = "data_output";
        if !std::path::Path::new(output_dir).exists() {
            std::fs::create_dir(output_dir).unwrap_or_else(|_| {
                error!("Failed to create output directory: {}", output_dir);
            });
        }

        let now = std::time::SystemTime::now();
        let now: chrono::DateTime<chrono::Utc> = now.into();
        let now = now.format("%Y-%m-%d_%H-%M-%S");

        let file_path = if homing {
            format!(
                "{}/repeatability_homing_tool_{}_data_{}.txt",
                output_dir, self.current_tool, now
            )
        } else {
            format!(
                "{}/repeatability_tool_{}_data_{}.txt",
                output_dir, self.current_tool, now
            )
        };

        match std::fs::File::create(&file_path) {
            Ok(mut file) => {
                use std::io::Write;

                // Write header
                if let Err(e) = writeln!(&mut file, "X,Y") {
                    error!("Failed to write header to file: {}", e);
                    return;
                }

                // Write each data point
                for ((x, y), (offset_x, offset_y)) in &self.repeatability {
                    if let Err(e) = writeln!(
                        &mut file,
                        "{:.6},{:.6},{:.6},{:.6}",
                        x, y, offset_x, offset_y
                    ) {
                        error!("Failed to write data point to file: {}", e);
                        return;
                    }
                }

                // Write statistics
                if let Err(e) = writeln!(&mut file, "\nStatistics: Tool {}", self.current_tool) {
                    error!("Failed to write to file: {}", e);
                    return;
                }

                let stats = [
                    // ("Median", median_x, median_y),
                    // ("Mean", mean_x, mean_y),
                    ("StdDev", std_dev_x, std_dev_y),
                    // ("Min", min_x, min_y),
                    // ("Max", max_x, max_y),
                    ("Range", range_x, range_y),
                ];

                for (name, x_val, y_val) in stats {
                    if let Err(e) = writeln!(&mut file, "{}: X={:.6}, Y={:.6}", name, x_val, y_val)
                    {
                        error!("Failed to write stats to file: {}", e);
                        return;
                    }
                }

                info!("Repeatability data saved to {}", file_path);
            }
            Err(e) => {
                error!("Failed to create file for repeatability data: {}", e);
            }
        }
    }

    pub fn repeatability_count_mut(&mut self) -> &mut usize {
        &mut self.check_repeatability
    }
}

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

            let mut move_x = x;
            let mut move_y = y;

            if self.options.auto_offset_settings.swap_axes {
                std::mem::swap(&mut move_x, &mut move_y);
            }
            if self.options.auto_offset_settings.mirror_axes.0 {
                // debug!("Mirroring X axis");
                move_x *= -1.0;
            }
            if self.options.auto_offset_settings.mirror_axes.1 {
                // debug!("Mirroring Y axis");
                move_y *= -1.0;
            }

            match self.auto_offset.auto_offset_type() {
                AutoOffsetType::None => {}
                AutoOffsetType::SingleTool => self._auto_offset_single(ui, (move_x, move_y)),
                AutoOffsetType::AllTools => todo!(),
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
        // unimplemented!()
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

        if stop {
            if self.auto_offset.check_repeatability == 0 {
                let t = self.auto_offset.auto_offset_type;
                self.auto_offset.stop();

                let Some(pos) = self.get_position() else {
                    warn!("No position data available");
                    return;
                };
                self.auto_offset
                    .repeatability
                    .push(((pos.0, pos.1), (x, y)));

                self.auto_offset
                    .process_repeatibility(matches!(t, AutoOffsetType::HomingTest));

                return;
            } else {
                debug!("Found center, adding to repeatability data");

                self.auto_offset.check_repeatability -= 1;

                let now = std::time::SystemTime::now();
                let now: chrono::DateTime<chrono::Utc> = now.into();
                let now = now.format("%Y-%m-%d_%H-%M-%S");

                let path_dir = format!(
                    "data_output/screenshots_T{}_{}",
                    self.auto_offset.current_tool, now
                );

                if !std::path::Path::new("data_output").exists() {
                    std::fs::create_dir("data_output").unwrap_or_else(|_| {
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

                let Some(pos) = self.get_position() else {
                    warn!("No position data available");
                    return;
                };
                self.auto_offset
                    .repeatability
                    .push(((pos.0, pos.1), (x, y)));

                match self.auto_offset.auto_offset_type {
                    AutoOffsetType::RepeatabilityTest => {
                        // self.dropoff_tool();
                        self.move_to_position((30., 220.), true);
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
