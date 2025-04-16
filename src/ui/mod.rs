// pub mod app;
pub mod auto_offset;
pub mod auto_offset_types;
pub mod data_labeling;
pub mod klipper_ui;
pub mod options;
pub mod preprocess_ui;
pub mod ui_types;
pub mod utils;
pub mod webcam_controls;

use tracing_subscriber::field::debug;
use ui_types::*;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{Align, Button, Color32, Label, Layout, RichText, Vec2};
use egui_extras::StripBuilder;

use crate::vision::{VisionSettings, WebcamMessage};

/// New
impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        let mut out: Self = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        let mut webcam_settings = out.webcam_settings_mutex.lock().unwrap();
        *webcam_settings = out.vision_settings;
        drop(webcam_settings);

        if let Err(e) = crate::appconfig::read_options_from_file("config.toml", &mut out.options) {
            error!("Failed to read options from file: {}", e);
        }

        if let Ok(data) = crate::saved_data::SavedData::load_from_file("saved_data.toml") {
            out.camera_pos = Some(data.camera_position);
        }

        out
    }
}

/// controls
impl App {
    fn controls(&mut self, ui: &mut egui::Ui) {
        #[cfg(feature = "nope")]
        if self.klipper.is_none() {
            ui.label("No Klipper connection");
            ui.label(format!("Printer URL: {}", self.options.printer_url));

            if ui.button("Connect").clicked() || !self.tried_startup_connection {
                self.tried_startup_connection = true;

                let Ok(url) = url::Url::parse(&self.options.printer_url) else {
                    self.errors.push("Invalid URL".to_string());
                    return;
                };

                debug!("Connecting to Klipper at {}", url.to_string());

                let mut klipper = match crate::klipper_protocol::KlipperProtocol::new(url) {
                    Ok(klipper) => klipper,
                    Err(e) => {
                        self.errors.push(format!("Failed to connect: {}", e));
                        return;
                    }
                };

                self.klipper = Some(klipper);

                if let Err(e) = self.klipper.as_mut().unwrap().fetch_position() {
                    self.errors.push("Failed to get position".to_string());
                }

                match self.klipper.as_mut().unwrap().get_tool_offsets() {
                    Ok(offsets) => {
                        self.tool_offsets = offsets;
                    }
                    Err(e) => {
                        self.errors
                            .push(format!("Failed to get tool offsets: {}", e));
                    }
                }
            }

            return;
        };

        /// Auto Offset
        ui.horizontal(|ui| {
            let button = egui::Button::new(RichText::new("Locate Single Nozzle").size(16.));
            let button = if matches!(
                self.auto_offset.auto_offset_type(),
                auto_offset::AutoOffsetType::SingleTool
            ) {
                button.fill(Color32::from_rgb(50, 158, 244))
            } else {
                button
            };
            if ui.add(button).clicked() {
                if matches!(
                    self.auto_offset.auto_offset_type(),
                    auto_offset::AutoOffsetType::SingleTool
                ) {
                    self.auto_offset.stop();
                } else {
                    if let Some((x, y, _)) = self.get_position() {
                        if let Some(tool) = self.active_tool {
                            self.auto_offset.start_single((x, y), tool as i32);
                        }
                    }
                }
            }

            let button = egui::Button::new(RichText::new("Locate All Nozzles").size(16.));
            let button = if matches!(
                self.auto_offset.auto_offset_type(),
                auto_offset::AutoOffsetType::AllTools
            ) {
                button.fill(Color32::from_rgb(50, 158, 244))
            } else {
                button
            };
            if ui.add(button).clicked() {
                if let Some((x, y, _)) = self.get_position() {
                    if let Some(tool) = self.active_tool {
                        if let Some((x, y, _)) = self.get_position() {
                            self.auto_offset.start_all_tools((x, y));
                        }
                    }
                }
            }

            let button = egui::Button::new(RichText::new("Repeatability Test").size(16.));
            let button = if matches!(
                self.auto_offset.auto_offset_type(),
                crate::ui::auto_offset::AutoOffsetType::RepeatabilityTest
            ) {
                button.fill(Color32::from_rgb(50, 158, 244))
            } else {
                button
            };
            if ui.add(button).clicked() {
                if let Some((x, y, _)) = self.get_position() {
                    if let Some(t) = self.active_tool {
                        match self.auto_offset.auto_offset_type() {
                            auto_offset::AutoOffsetType::RepeatabilityTest => {
                                self.auto_offset.stop()
                            }
                            _ => {
                                debug!("Starting repeatability test");
                                if let Some((x, y, _)) = self.get_position() {
                                    self.auto_offset.start_repeatability((x, y), t as i32);
                                }
                            }
                        }
                    }
                }
            }

            let button = egui::Button::new(RichText::new("Homing Test").size(16.));
            let button = if matches!(
                self.auto_offset.auto_offset_type(),
                crate::ui::auto_offset::AutoOffsetType::HomingTest
            ) {
                button.fill(Color32::from_rgb(50, 158, 244))
            } else {
                button
            };
            if ui.add(button).clicked() {
                if let Some((x, y, _)) = self.get_position() {
                    if let Some(t) = self.active_tool {
                        debug!("Starting repeatability test");
                        if let Some((x, y, _)) = self.get_position() {
                            self.auto_offset.start_homing((x, y), t as i32);
                        }
                    }
                }
            }

            ui.label("Repeatability Count: ");
            let resp = ui.add(
                egui::DragValue::new(self.auto_offset.repeatability_count_mut())
                    .speed(0.5)
                    .range(0..=20),
            );
            self::utils::make_scrollable(
                ui,
                resp,
                self.auto_offset.repeatability_count_mut(),
                1,
                // Some(0),
            );

            // let x = self.auto_offset.repeatability_count_mut();
            // if x > 0 {
            //     ui.label(format!("Tests remaining: {}", x));
            // }

            //
        });

        ui.separator();

        // /// screenshot
        // ui.horizontal(|ui| {
        //     if ui
        //         .button(RichText::new("Save Screenshot").size(16.))
        //         .clicked()
        //     {
        //         // self.save_screenshot();
        //         let _ = self
        //             .channel_to_vision
        //             .as_mut()
        //             .unwrap()
        //             .try_send(crate::vision::WebcamCommand::SaveScreenshot(None));
        //     }
        // });
        // ui.separator();

        /// home
        ui.horizontal(|ui| {
            if ui.button(RichText::new("Home All").size(16.)).clicked() {
                self.home_all();
            }

            if ui.button(RichText::new("Home XY").size(16.)).clicked() {
                self.home_xy();
            }

            let enabled = self
                .klipper_status_frame
                .as_ref()
                .map(|s| s.motors_enabled.0 || s.motors_enabled.1 || s.motors_enabled.2)
                .unwrap_or(false);

            if ui
                .add_enabled(
                    enabled,
                    egui::Button::new(RichText::new("Disable Motors").size(16.)),
                )
                .clicked()
            {
                // self.home_z();
                self.disable_motors();
            }
            //
        });
        ui.separator();

        /// tools
        ui.horizontal(|ui| {
            if ui.button(RichText::new("Dropoff Tool").size(16.)).clicked() {
                self.dropoff_tool();
            }

            for t in 0..4 {
                let but = egui::Button::new(RichText::new(format!("T{}", t)).size(16.));
                let but = if self.active_tool == Some(t) {
                    but.fill(egui::Color32::from_rgb(50, 158, 244))
                } else {
                    but
                };

                if ui.add(but).clicked() {
                    self.pickup_tool(t as i32, true);
                    if let Some(pos) = self.camera_pos {
                        self.move_to_position(pos, true);
                    } else {
                        self.errors.push("No camera position saved".to_string());
                    }
                }
            }
            //
        });
        ui.separator();

        /// Camera Offset
        ui.horizontal(|ui| {
            if let Some((x, y, z)) = self.get_position() {
                if ui
                    .add(
                        egui::Button::new(RichText::new("Save camera Position").size(16.))
                            .fill(egui::Color32::from_rgb(251, 149, 20)),
                    )
                    .clicked()
                {
                    if let Some((x, y, _)) = self.fetch_position() {
                        self.camera_pos = Some((x, y));
                        debug!("Camera position saved: ({:?}, {:?})", x, y);

                        let mut saved_data =
                            crate::saved_data::SavedData::load_from_file("saved_data.toml")
                                .unwrap_or_default();

                        saved_data.camera_position = (x, y);

                        saved_data
                            .save_to_file("saved_data.toml")
                            .unwrap_or_else(|e| {
                                error!("Failed to save camera position: {}", e);
                            });
                    } else {
                        self.errors.push("Failed to get position".to_string());
                        self.camera_pos = None;
                    }
                }

                if ui
                    .add(
                        egui::Button::new(RichText::new("Move to camera").size(16.))
                            .fill(egui::Color32::from_rgb(50, 158, 244)),
                    )
                    .clicked()
                {
                    if let Some(pos) = self.camera_pos {
                        self.move_to_position(pos, true);
                    } else {
                        self.errors.push("No camera position saved".to_string());
                    }
                }

                if let Some((camera_x, camera_y)) = self.camera_pos {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Camera Position: ({:.4}, {:.4})",
                                camera_x, camera_y
                            ))
                            .size(16.),
                        );

                        let (offset_x, offset_y, _) = if let Some(t) = self.active_tool {
                            self.tool_offsets.get(t).copied().unwrap_or_else(|| {
                                error!("Failed to get tool offsets");
                                (0., 0., 0.)
                            })
                        } else {
                            (0., 0., 0.)
                        };

                        let cx = x - camera_x - offset_x;
                        let cy = y - camera_y - offset_y;
                        ui.label(
                            RichText::new(format!(
                                "Diff from Camera (GCode): {:.4}, {:.4}",
                                cx, cy
                            ))
                            .size(16.),
                        );

                        #[cfg(feature = "nope")]
                        if let Some(guess) = self.running_average.current_guess() {
                            let (gx, gy, _) = guess;

                            debug!("Guess: ({:.4}, {:.4})", gx, gy);

                            let gx = gx - camera_x - offset_x;
                            let gy = gy - camera_y - offset_y;

                            ui.label(
                                RichText::new(format!(
                                    "Diff from Camera (Located): {:.4}, {:.4}",
                                    gx, gy
                                ))
                                .size(16.),
                            );
                        } else {
                            ui.label(
                                RichText::new(format!(
                                    "Diff from Camera (Located): -.----, -.----"
                                ))
                                .size(16.),
                            );
                        }
                    });
                } else {
                    ui.label(RichText::new(format!("No Camera Position")).size(16.));
                }
            };
        });
        ui.separator();

        /// Test positions
        #[cfg(feature = "nope")]
        ui.horizontal(|ui| {
            #[cfg(feature = "nope")]
            let test_positions = [
                (283.9591, 25.84169999999999),
                (284.16, 25.02),
                (283.86, 23.52),
                (284.36, 23.52),
                //
            ];

            let test_positions = [(0., 0.5), (0.5, 0.), (0.5, 0.5), (-1., -1.)];

            if let Some(camera_pos) = self.camera_pos {
                for (i, pos) in test_positions.iter().enumerate() {
                    if ui
                        .add(Button::new(RichText::new(format!("Pos {}", i)).size(15.)))
                        .clicked()
                    {
                        let pos = (camera_pos.0 + pos.0, camera_pos.1 + pos.1);
                        self.move_to_position(pos, true);
                    }
                }
            }

            #[cfg(feature = "nope")]
            if ui
                .add(Button::new(RichText::new("Test move (0.5, 0.5)").size(15.)))
                .clicked()
            {
                let x = 5.0;
                self.move_relative((x, x), true);
            }
        });
        #[cfg(feature = "nope")]
        ui.separator();

        let Some((x, y, z)) = self.get_position() else {
            ui.label("No position");
            return;
        };

        // egui::ScrollArea::neither()
        // .max_height(100.)
        #[cfg(feature = "nope")]
        egui::Frame::group(ui.style())
            // .max_width()
            .show(ui, |ui| {
                ui.set_height(100.);
                StripBuilder::new(ui)
                    .sizes(egui_extras::Size::exact(50.), 2)
                    // .sizes(egui_extras::Size::relative(0.5), 2)
                    .vertical(|mut strip| {
                        strip.strip(|builder| {
                            self.movement_buttons(builder, Axis::X, (x, y, z));
                        });
                        strip.strip(|builder| {
                            self.movement_buttons(builder, Axis::Y, (x, y, z));
                        });
                    });
            });

        #[cfg(feature = "nope")]
        egui::Frame::group(ui.style()).show(ui, |ui| {
            StripBuilder::new(ui)
                .sizes(egui_extras::Size::exact(100.), 2)
                .vertical(|mut strip| {
                    strip.strip(|builder| {
                        self.movement_buttons(builder, Axis::X, (x, y, z));
                    });
                    strip.strip(|builder| {
                        self.movement_buttons(builder, Axis::Y, (x, y, z));
                    });
                });
        });

        //
    }

    fn movement_controls(&mut self, ui: &mut egui::Ui) {
        let Some((x, y, z)) = self.get_position() else {
            ui.label("No position");
            return;
        };

        #[cfg(feature = "nope")]
        ui.horizontal(|ui| {
            let layout = Layout::default()
                .with_main_justify(true)
                .with_cross_justify(true)
                .with_cross_align(Align::Center)
                .with_main_align(Align::Center)
                // .with_cross_align(egui::Align::Center)
                ;

            // let layout = Layout::left_to_right(Align::Center);

            ui.with_layout(layout, |ui| {
                // self.position_labels(ui);
                // ui.group(|ui| {
                //     for i in 0..3 {
                //     }
                // });

                {
                    StripBuilder::new(ui)
                        .sizes(egui_extras::Size::exact(100.), 3)
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.label(RichText::new(format!("X: {}", 0)).size(18.).strong());
                            });
                            strip.cell(|ui| {
                                ui.label(RichText::new(format!("Y: {}", 1)).size(18.).strong());
                            });
                            strip.cell(|ui| {
                                ui.label(RichText::new(format!("Z: {}", 2)).size(18.).strong());
                            });
                        });
                }
            });
        });

        ui.horizontal(|ui| {
            self.position_labels(ui);
        });

        ui.group(|ui| {
            ui.set_height(100.);

            let layout = Layout::default()
                .with_cross_justify(true)
                // .with_cross_align(egui::Align::Center)
                ;

            ui.with_layout(layout, |ui| {
                StripBuilder::new(ui)
                    .sizes(egui_extras::Size::relative(0.33), 3)
                    .vertical(|mut strip| {
                        strip.strip(|builder| {
                            self.movement_buttons(builder, Axis::X, (x, y, z));
                        });
                        strip.strip(|builder| {
                            self.movement_buttons(builder, Axis::Y, (x, y, z));
                        });
                        strip.strip(|builder| {
                            self.movement_buttons(builder, Axis::Z, (x, y, z));
                        });
                    });
            });
        });
    }

    fn position_labels(&mut self, ui: &mut egui::Ui) {
        let Some((x, y, z)) = self.get_position() else {
            ui.label("No position");
            return;
        };

        fn label(ui: &mut egui::Ui, (x, y, z): (f64, f64, f64), axis: Axis) {
            let pos = match axis {
                Axis::X => x,
                Axis::Y => y,
                Axis::Z => z,
            };
            // let text = format!("{}: {:.4}", axis.to_str().to_uppercase(), pos);
            // ui.label(RichText::new(text).size(18.0).strong());

            ui.group(|ui| {
                ui.set_width(100.);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{}: ", axis.to_str().to_uppercase()))
                            .size(18.)
                            .strong(),
                    );

                    // let layout = egui::Layout::right_to_left(egui::Align::Center);

                    // ui.with_layout(layout, |ui| {
                    ui.label(RichText::new(format!("{:.4}", pos)).strong().size(18.));
                    // });
                });
            });
        }

        StripBuilder::new(ui)
            .sizes(egui_extras::Size::exact(100.), 3)
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    label(ui, (x, y, z), Axis::X);
                });
                strip.cell(|ui| {
                    label(ui, (x, y, z), Axis::Y);
                });
                strip.cell(|ui| {
                    label(ui, (x, y, z), Axis::Z);
                });
            });
    }

    // #[cfg(feature = "nope")]
    fn movement_buttons<'a>(
        &mut self,
        builder: egui_extras::StripBuilder<'a>,
        axis: Axis,
        (x, y, z): (f64, f64, f64),
    ) {
        let steps = match axis {
            Axis::Z => [0.01, 0.02, 0.05, 0.1, 0.5],
            // _ => [0.01, 0.1, 0.5, 1., 5.],
            _ => [0.005, 0.01, 0.1, 0.5, 5.],
        };
        // let steps = [0.01, 0.1, 5., 10., 15.];
        let button_width = 60.;

        let pos = match axis {
            Axis::X => x,
            Axis::Y => y,
            Axis::Z => z,
        };

        builder
            .sizes(egui_extras::Size::exact(button_width), steps.len())
            // .size(egui_extras::Size::exact(100.))
            .size(egui_extras::Size::exact(50.))
            // .sizes(egui_extras::Size::relative(0.1), steps.len())
            .sizes(egui_extras::Size::exact(button_width), steps.len())
            .cell_layout(egui::Layout::default().with_cross_align(egui::Align::Center))
            .horizontal(|mut strip| {
                let mut strip = self.button_range(strip, axis, &steps, true);

                strip.cell(|ui| {
                    let layout = egui::Layout::default()
                        .with_cross_align(egui::Align::Center)
                        .with_main_justify(true);

                    let text = format!("{}: {:.4}", axis.to_str(), pos);

                    // ui.set_width(100.);
                    // ui.set_height(40.);

                    let text = axis.to_str().to_uppercase();
                    let button = egui::Button::new(RichText::new(&text).size(16.0).strong())
                        .fill(Color32::RED)
                        .min_size(egui::vec2(60., 40.));

                    ui.add(button);

                    // ui.with_layout(layout, |ui| {
                    //     //
                    // });

                    // egui::Frame::NONE
                    //     .fill(Color32::RED)
                    //     .corner_radius(3.)
                    //     // .fill(Color32::RED)
                    //     .show(ui, |ui| {
                    //         ui.with_layout(layout, |ui| {
                    //             ui.label(
                    //                 RichText::new(format!("{}: {:.2}", axis.to_str(), pos))
                    //                     .size(20.)
                    //                     .strong(),
                    //             );
                    //         });
                    //     });
                });

                let mut strip = self.button_range(strip, axis, &steps, false);
            });
    }

    fn button_range<'a, 'b>(
        &mut self,
        mut strip: egui_extras::Strip<'a, 'b>,
        axis: Axis,
        steps: &[f64],
        neg: bool,
    ) -> egui_extras::Strip<'a, 'b> {
        for i in 0..steps.len() {
            let step = if neg {
                steps[steps.len() - i - 1]
            } else {
                steps[i]
            };
            strip.cell(|ui| {
                let text = if neg {
                    // format!("{:.2}", -step)
                    format!("{}", -step)
                } else {
                    // format!("+{:.2}", step)
                    format!("+{}", step)
                };
                let button = egui::Button::new(RichText::new(text).size(16.0).strong())
                    .fill(egui::Color32::from_rgb(50, 158, 244))
                    .min_size(egui::vec2(60., 40.));

                // let layout = egui::Layout::default().with_cross_align(egui::Align::Center);

                // let button = ui.with_layout(layout, |ui| ui.add(button));

                if ui.add(button).clicked() {
                    // eprintln!("Clicked on X: {}", -step);

                    let step = if neg { -step } else { step };
                    self.move_axis_relative(axis, step, true);
                }
            });
        }

        strip
    }

    #[cfg(feature = "nope")]
    fn offset_adjust(&mut self, ui: &mut egui::Ui) {
        let Some(tool) = self.active_tool else {
            ui.label("No active tool");
            return;
        };

        let (offset_x, offset_y, offset_z) = self.tool_offsets[tool];

        ui.horizontal(|ui| {
            ui.label(format!("Tool {} offsets:", tool));
            ui.label(format!("X: {:.3}", offset_x));
            ui.label(format!("Y: {:.3}", offset_y));
            ui.label(format!("Z: {:.3}", offset_z));
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Adjust:");
            ui.radio_value(&mut self.offset_axis, Axis::X, "X");
            ui.radio_value(&mut self.offset_axis, Axis::Y, "Y");
            // ui.radio_value(&mut self.offset_axis, Axis::Z, "Z");
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Offset:");
            ui.add(
                egui::DragValue::new(&mut self.offset_value)
                    .range(-0.1..=0.1)
                    .speed(0.01)
                    .fixed_decimals(3),
            );
        });

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("Apply").clicked() {
                let axis = match self.offset_axis {
                    Axis::X => 0,
                    Axis::Y => 1,
                    Axis::Z => 2,
                };

                if let Err(e) = klipper.adjust_tool_offset(tool, axis, self.offset_value) {
                    self.errors
                        .push(format!("Failed to adjust tool {} offset: {}", tool, e));
                } else {
                    match axis {
                        0 => {
                            self.tool_offsets[tool].0 += self.offset_value;
                        }
                        1 => {
                            self.tool_offsets[tool].1 += self.offset_value;
                        }
                        // 2 => {
                        //     self.tool_offsets[tool].2 += self.offset_value;
                        // }
                        _ => {}
                    }
                }

                //
            }

            if ui
                .add(
                    egui::Button::new("Apply offset from camera")
                        .fill(egui::Color32::from_rgb(50, 158, 244)),
                )
                .clicked()
            {
                if let Some((camera_x, camera_y)) = self.camera_pos {
                    let (offset_x, offset_y, _) = if let Some(tool) = self.active_tool {
                        self.tool_offsets[tool]
                    } else {
                        (0., 0., 0.)
                    };

                    let (x, y, _) = klipper.get_position().unwrap_or((0., 0., 0.));

                    let x = x - camera_x - offset_x;
                    let y = y - camera_y - offset_y;

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
                    self.move_to_position(cam_pos, true)
                }
            }
        });
        //
    }

    fn webcam(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                egui::Grid::new("Filter Controls").show(ui, |ui| {
                    // if ui.button("Clear Running Average").clicked() {
                    //     self.running_average.clear();
                    // }
                    // ui.end_row();

                    // if let Some((confidence, (c_x, c_y, c_r))) = self.running_average.confidence() {
                    //     //
                    // }

                    // if let Some((x, y, r)) = self.running_average.current_guess() {
                    //     ui.label(
                    //         RichText::new(format!(
                    //             "Current Guess: ({:.4}, {:.4}, r = {:.1})",
                    //             x, y, r
                    //         ))
                    //         .monospace(),
                    //     );
                    // } else {
                    //     ui.label(RichText::new("Current Guess: None").monospace());
                    // }

                    // let (confidence, result) = self.running_average.get_result();
                    // // let confidence = self.running_average.calculate_confidence();
                    // ui.label(format!("Confidence: {:.3}", confidence));
                    // ui.end_row();
                    // if let Some(result) = result {
                    //     ui.label(format!("Result: ({:.3}, {:.3})", result.0, result.1));
                    // } else {
                    //     ui.label("Result: None");
                    // }

                    // if let Some((_, moe)) = self.running_average.calculate_margin_of_error() {
                    //     ui.label(format!("Margin of Error: ({:.3}, {:.3})", moe.0, moe.1));
                    // } else {
                    //     ui.label("Margin of Error: None");
                    // }
                    // ui.end_row();

                    // ui.separator();
                    // ui.end_row();

                    // if let Some((_, tgt)) = self.data_labeling.target {
                    //     ui.label(
                    //         RichText::new(format!("Target: ({:.1}, {:.1})", tgt.x, tgt.y)), // .size(14.),
                    //     );
                    // } else {
                    //     ui.label(RichText::new("Target: ").size(14.));
                    // }
                    // ui.end_row();

                    // if let Some((x, y, radius)) = self.current_located_nozzle {
                    //     ui.label(
                    //         RichText::new(format!("Located: ({:.1}, {:.1}), {:.0}", x, y, radius))
                    //             .size(14.),
                    //     );
                    // } else {
                    //     ui.label(RichText::new("Located: ").size(14.));
                    // }

                    // ui.end_row();

                    // ui.separator();
                    // ui.end_row();

                    ui.horizontal(|ui| {
                        ui.label("Scale: ");
                        let r0 = ui.radio_value(&mut self.options.camera_scale, 0.5, "x0.5");
                        let r1 = ui.radio_value(&mut self.options.camera_scale, 1.0, "x1.0");

                        if r0.changed() || r1.changed() {
                            // let r = ui.ctx().input(|i: &egui::InputState| i.screen_rect());

                            /// 1371, 848
                            /// 1714, 1223
                            // debug!("r = {:?}", r);
                            let s = match self.options.camera_scale {
                                0.5 => Vec2::new(1200., 850.),
                                1.0 => Vec2::new(1750., 1250.),
                                _ => unimplemented!(),
                            };

                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::InnerSize(s));

                            //
                        }
                    });

                    ui.end_row();

                    ui.separator();
                    ui.end_row();

                    self.webcam_controls(ui);
                });
            });

            self._webcam(ui);
        });
        //
    }

    fn _webcam(&mut self, ui: &mut egui::Ui) {
        let texture = match &self.webcam_texture {
            Some(texture) => texture,
            None => {
                // let image = egui::ColorImage::new([1280, 800], egui::Color32::from_gray(220));
                let image = egui::ColorImage::new(
                    [
                        // self.options.camera_size.0 as usize * 2,
                        // self.options.camera_size.1 as usize * 2,
                        self.options.camera_size.0 as usize,
                        self.options.camera_size.1 as usize,
                    ],
                    egui::Color32::from_gray(220),
                );

                let texture = ui
                    .ctx()
                    .load_texture("camera_texture", image, Default::default());

                self.webcam_texture = Some(texture.clone());

                let (tx_to_ui, rx_to_ui) = crossbeam_channel::bounded(1);
                self.channel_to_ui = Some(rx_to_ui);

                let (tx_to_vision, rx_to_vision) = crossbeam_channel::bounded(10);
                self.channel_to_vision = Some(tx_to_vision);

                crate::vision::spawn_locator_thread(
                    ui.ctx().clone(),
                    texture.clone(),
                    0,
                    rx_to_vision,
                    tx_to_ui,
                    self.webcam_settings_mutex.clone(),
                    // self.options.camera_size,
                    self.selected_camera_format,
                );

                &self.webcam_texture.as_ref().unwrap()
            }
        };

        let size = egui::Vec2::new(
            // crate::webcam::Webcam::SIZE.0 as f32,
            // crate::webcam::Webcam::SIZE.1 as f32,
            self.options.camera_size.0 as f32,
            self.options.camera_size.1 as f32,
        );
        // let size = size / 1.;
        // let size = size / 2.;
        // let size = size / 1.5;
        let size = size * self.options.camera_scale as f32;

        // let c = ui.cursor();
        let img = egui::Image::from_texture((texture.id(), size))
            .fit_to_exact_size(size)
            .max_size(size)
            // .rounding(egui::Rounding::same(4.))
            .sense(egui::Sense::click());

        let resp = ui.add(img);

        let rect = resp.rect;

        // debug!("rect.min = ({:.1}, {:.1})", rect.min.x, rect.min.y);

        if resp.clicked() {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                if let Some(tool) = self.active_tool {
                    // offset pos by cursor

                    // debug!("c = {:?}", c.min);
                    // let pos = egui::Pos2::new(pos.x - c.min.x, pos.y - c.min.y);
                    let pos = egui::Pos2::new(pos.x - rect.min.x, pos.y - rect.min.y);

                    self.data_labeling.target = Some((tool, pos));
                }
            }
        }

        /// scroll to adjust pointer size
        if resp.hovered() {
            let delta = ui.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::MouseWheel {
                        unit: _,
                        delta,
                        modifiers,
                    } => Some(*delta),
                    _ => None,
                })
            });
            if let Some(delta) = delta {
                if delta.y > 0. {
                    self.vision_settings.target_radius += 0.5;
                } else if delta.y < 0. && self.vision_settings.target_radius > 0. {
                    self.vision_settings.target_radius -= 0.5;
                }
            }
        }

        /// right click to save screenshot
        if let Some((_, pos)) = self.data_labeling.target {
            let painter = ui.painter_at(resp.rect);

            // let pos = pos + egui::Vec2::from([c.min.x, c.min.y]);
            let pos = pos + egui::Vec2::from([rect.min.x, rect.min.y]);

            let radius = self.vision_settings.target_radius as f32;

            painter.circle_stroke(pos, radius, egui::Stroke::new(1.0, egui::Color32::RED));

            painter.line(
                vec![pos + Vec2::new(0., radius), pos + Vec2::new(0., -radius)],
                egui::Stroke::new(1.0, egui::Color32::RED),
            );

            painter.line(
                vec![pos + Vec2::new(radius, 0.), pos + Vec2::new(-radius, 0.)],
                egui::Stroke::new(1.0, egui::Color32::RED),
            );

            if ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary)) {
                self.data_labeling.num_screens = 1;
            }

            if self.data_labeling.num_screens > 0 {
                self.data_labeling.num_screens -= 1;

                debug!("pos = ({:.1}, {:.1})", pos.x, pos.y);
                debug!("c.min = ({:.1}, {:.1})", rect.min.x, rect.min.y);

                let x = pos.x as f64 - rect.min.x as f64;
                let y = pos.y as f64 - rect.min.y as f64;

                let x = x / self.options.camera_scale;
                let y = y / self.options.camera_scale;

                self.channel_to_vision
                    .as_mut()
                    .unwrap()
                    .try_send(crate::vision::WebcamCommand::SaveScreenshot(
                        Some((x, y)),
                        None,
                    ))
                    .unwrap_or_else(|e| {
                        error!("Failed to send screenshot command: {}", e);
                    });
            }
            //
        }

        //
    }

    #[cfg(feature = "nope")]
    fn webcam_controls(&mut self, ui: &mut egui::Ui) {
        egui_probe::Probe::new(&mut self.webcam_settings).show(ui);

        if self.webcam_settings != self.webcam_settings_prev {
            let mut settings = self.webcam_settings_mutex.lock().unwrap();
            *settings = self.webcam_settings;
            self.webcam_settings_prev = self.webcam_settings.clone();
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    #[cfg(feature = "nope")]
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        /// start async thread
        if !self.klipper_started {
            debug!("starting klipper thread");
            let url = url::Url::parse(&format!("{}", self.options.printer_url)).unwrap();
            let url = url::Url::parse(&format!("ws://{}:7125/websocket", url.host_str().unwrap()))
                .unwrap();

            // debug!("url = {}", url);

            let sender_pos = self.inbox.sender();

            let (tx, rx) = crossbeam_channel::bounded(1);

            std::thread::spawn(move || {
                // let rt = tokio::runtime::Runtime::new().unwrap();
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(4)
                    .enable_all()
                    .build()
                    .unwrap();

                rt.block_on(async move {
                    let mut klipper = crate::klipper_async::KlipperConn::new(url, sender_pos, rx)
                        .await
                        .unwrap();
                    klipper.run().await.unwrap();
                });
            });

            self.klipper_started = true;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // while let Some(msg) = self.inbox_position.read(ui).next() {
            //     debug!("Got message: {:?}", msg);
            // }

            if let Some(pos) = self.inbox.read(ui).last() {
                // self.last_position = pos;
                unimplemented!()
            }

            ui.label("Current Position:");
            ui.label(format!("X: {:.3}", self.last_position.0));
            ui.label(format!("Y: {:.3}", self.last_position.1));
            ui.label(format!("Z: {:.3}", self.last_position.2));

            // ui

            //
        });
    }

    // #[cfg(feature = "nope")]
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // if cfg!(debug_assertions) && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        //     ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        // }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        /// Init klipper
        if !self.klipper_started {
            // self.errors.push("Starting klipper".to_string());

            debug!("starting klipper thread");
            let url = url::Url::parse(&format!("{}", self.options.printer_url)).unwrap();
            let url = url::Url::parse(&format!("ws://{}:7125/websocket", url.host_str().unwrap()))
                .unwrap();

            // debug!("url = {}", url);

            let sender_pos = self.inbox.sender();

            let (mut tx, rx) = tokio::sync::mpsc::channel(2);

            tx.blocking_send(crate::klipper_async::KlipperCommand::GetToolOffsets)
                .unwrap_or_else(|e| {
                    error!("Failed to send command: {}", e);
                });

            self.klipper_tx = Some(tx);

            let (tx2, mut rx2) = tokio::sync::oneshot::channel();

            std::thread::spawn(move || {
                // let rt = tokio::runtime::Runtime::new().unwrap();
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(3)
                    .enable_all()
                    .build()
                    .unwrap();

                rt.block_on(async move {
                    let mut klipper =
                        crate::klipper_async::KlipperConn::new(url, sender_pos, rx, tx2)
                            .await
                            .unwrap();
                    klipper.run().await.unwrap();
                });
            });

            let status = loop {
                if let Ok(status) = rx2.try_recv() {
                    break status;
                };
            };

            self.klipper_status = Some(status);

            self.klipper_started = true;
        }

        self.inbox.set_ctx(ctx);
        while let Some(msg) = self.inbox.read_without_ctx().next() {
            match msg {
                crate::klipper_async::KlipperMessage::Position(pos) => self.last_position = pos,
                // crate::klipper_async::KlipperMessage::AxesHomed((x, y, z)) => todo!(),
                crate::klipper_async::KlipperMessage::KlipperError(e) => {
                    error!("Klipper error: {}", e);
                    self.errors.push(e.to_string());
                }
                crate::klipper_async::KlipperMessage::ToolOffsets(offsets) => {
                    self.tool_offsets = offsets
                }
                _ => {
                    debug!("Unhandled message: {:?}", msg);
                }
            }
        }

        // if let Some(status) = self.klipper_status.as_ref()
        if let Some(status) = self.klipper_status.as_ref() {
            if let Ok(status) = status.try_read() {
                self.klipper_status_frame = Some(status.clone());
            } else {
                self.klipper_status_frame = None;
            }
        } else {
            self.klipper_status_frame = None;
        }

        if let Some(rx) = self.channel_to_ui.as_mut() {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    WebcamMessage::FoundNozzle(pos) => {
                        // debug!("Found nozzle: {:?}", pos);
                        // self.running_average.push_position((pos.0, pos.1));
                        self.running_average.add_frame(Some(pos));
                        // self.current_located_nozzle = Some(pos);
                    }
                    WebcamMessage::NozzleNotFound => {
                        // debug!("Nozzle not found");
                        self.running_average.add_frame(None);
                        // self.current_located_nozzle = None;
                    }
                    WebcamMessage::CameraFormats(camera_formats) => {
                        // debug!("Got camera formats: {:?}", camera_formats.len());
                        self.camera_formats = camera_formats;
                        //
                    }
                }
            }
        }

        if self.camera_formats.len() == 0 && !self.camera_formats_request_sent {
            if let Some(tx) = self.channel_to_vision.as_ref() {
                if let Err(e) = tx.try_send(crate::vision::WebcamCommand::GetCameraFormats) {
                    error!("Failed to send camera formats command: {}", e);
                } else {
                    debug!("Sent get camera formats command");
                    self.camera_formats_request_sent = true;
                }
            }
        }

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, Tab::Webcam, "Webcam");
                // ui.selectable_value(&mut self.current_tab, Tab::DataLabeling, "Data Labeling");
                ui.selectable_value(&mut self.current_tab, Tab::Options, "Options");
            });
        });

        match self.current_tab {
            Tab::Webcam => {
                // #[cfg(feature = "nope")]
                egui::SidePanel::right("right")
                    .resizable(false)
                    .default_width(400.)
                    .show(ctx, |ui| {
                        // Let's show errors at the top of the panel
                        if !self.errors.is_empty() {
                            ui.heading("Errors");
                            ui.horizontal(|ui| {
                                if ui.button("Clear All").clicked() {
                                    self.errors.clear();
                                }

                                let error_count = self.errors.len();
                                ui.label(format!(
                                    "({} error{})",
                                    error_count,
                                    if error_count == 1 { "" } else { "s" }
                                ));
                            });

                            // Create a scrollable area for errors that won't take over the whole panel
                            egui::ScrollArea::vertical()
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    // Display errors in reverse order (newest first)
                                    for error in self.errors.iter().rev() {
                                        ui.label(
                                            egui::RichText::new(error)
                                                .color(egui::Color32::from_rgb(255, 100, 100)),
                                        );
                                        ui.separator();
                                    }
                                });

                            ui.separator();
                        }

                        // Original tool offsets section
                        if self.tool_offsets.is_empty() {
                            ui.label("No tool offsets");
                            return;
                        }

                        for t in 0..4 {
                            let (x, y, z) = self.tool_offsets[t];

                            ui.label(format!("Tool {} offsets:", t));
                            ui.label(format!("X: {:.3}", x));
                            ui.label(format!("Y: {:.3}", y));
                            ui.separator();
                        }
                    });

                egui::TopBottomPanel::bottom("bottom")
                    .resizable(false)
                    .default_height(600.)
                    .show(ctx, |ui| {
                        // if let Some(auto_offset) = self.auto_offset.take() {
                        //     self.auto_offset = self.auto_offset(ui, auto_offset);
                        // } else {
                        //     // self.offset_adjust(ui);
                        // }

                        self.auto_offset(ui);
                    });

                egui::TopBottomPanel::bottom("program controls")
                    .resizable(false)
                    .default_height(600.)
                    .show(ctx, |ui| {
                        // self.movement_controls(ui);

                        ui.vertical_centered(|ui| {
                            self.controls(ui);
                        });
                    });

                egui::TopBottomPanel::bottom("motion controls")
                    .resizable(false)
                    .default_height(600.)
                    .show(ctx, |ui| {
                        // ui.vertical_centered(|ui| {
                        self.movement_controls(ui);
                        // });
                    });

                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        self.webcam(ui);
                    });
                    ui.separator();

                    // ui.vertical_centered(|ui| {
                    //     self.controls(ui);
                    // });

                    //
                });
            }
            Tab::Options => {
                self.options(ctx);
            }
        }
    }
}
