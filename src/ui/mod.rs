pub mod ui_types;

use ui_types::*;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::RichText;
use egui_extras::StripBuilder;

use crate::vision::WebcamSettings;

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
        *webcam_settings = out.webcam_settings;
        drop(webcam_settings);

        // out.list_sort = Some((0, history_tab::SortOrder::Descending));

        // let filter = nucleo::Nucleo::new(
        //     nucleo::Config::DEFAULT,
        //     std::sync::Arc::new(|| {
        //         //
        //     }),
        //     Some(1),
        //     1,
        // );

        // let injector = filter.injector();

        // out.nucleo = Some(filter);
        // out.injector = Some(injector);

        // out.reload_db();

        out
    }
}

/// controls
impl App {
    fn controls(&mut self, ui: &mut egui::Ui) {
        let Some(klipper) = &mut self.klipper else {
            ui.label("No Klipper connection");

            if ui.button("Connect").clicked() || !self.tried_startup_connection {
                self.tried_startup_connection = true;
                let url = "http://192.168.0.245";
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

        /// home
        ui.horizontal(|ui| {
            if ui.button(RichText::new("Home All").size(16.)).clicked() {
                if let Err(e) = klipper.home_all() {
                    self.errors.push(format!("Failed to home all: {}", e));
                }
            }

            if ui.button(RichText::new("Home XY").size(16.)).clicked() {
                if let Err(e) = klipper.home_xy() {
                    self.errors.push(format!("Failed to home XY: {}", e));
                }
            }
            //
        });

        ui.separator();

        /// tools
        ui.horizontal(|ui| {
            if ui.button(RichText::new("Dropoff Tool").size(16.)).clicked() {
                if let Err(e) = klipper.dropoff_tool() {
                    self.errors.push(format!("Failed to dropoff tool: {}", e));
                } else {
                    self.active_tool = None;
                }
            }

            for t in 0..4 {
                let but = egui::Button::new(RichText::new(format!("T{}", t)).size(16.));
                let but = if self.active_tool == Some(t) {
                    but.fill(egui::Color32::from_rgb(50, 158, 244))
                } else {
                    but
                };

                if ui.add(but).clicked() {
                    if let Err(e) = klipper.pick_tool(t) {
                        self.errors
                            .push(format!("Failed to pick tool {}: {}", t, e));
                    } else {
                        self.active_tool = Some(t);
                    }
                    if let Some(pos) = self.camera_pos {
                        if let Err(e) = klipper.move_camera(pos) {
                            self.errors.push(format!("Failed to move to camera: {}", e));
                        }
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
            if let Some((x, y, z)) = klipper.get_position() {
                if ui
                    .add(
                        egui::Button::new(RichText::new("Save camera Position").size(16.))
                            .fill(egui::Color32::from_rgb(251, 149, 20)),
                    )
                    .clicked()
                {
                    self.camera_pos = Some((x, y));
                    // klipper.set_camera_pos(x, y);
                }

                if ui
                    .add(
                        egui::Button::new(RichText::new("Move to camera").size(16.))
                            .fill(egui::Color32::from_rgb(50, 158, 244)),
                    )
                    .clicked()
                {
                    if let Some(pos) = self.camera_pos {
                        if let Err(e) = klipper.move_camera(pos) {
                            self.errors.push(format!("Failed to move to camera: {}", e));
                        }
                    } else {
                        self.errors.push("No camera position saved".to_string());
                    }
                }

                if let Some((camera_x, camera_y)) = self.camera_pos {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Camera Position: ({:.2}, {:.2})",
                                camera_x, camera_y
                            ))
                            .size(16.),
                        );

                        let (offset_x, offset_y, _) = if let Some(t) = self.active_tool {
                            self.tool_offsets[t]
                        } else {
                            (0., 0., 0.)
                        };

                        let x = x - camera_x - offset_x;
                        let y = y - camera_y - offset_y;
                        ui.label(
                            RichText::new(format!("Diff from Camera: {:.3}, {:.3}", x, y))
                                .size(16.),
                        );
                    });
                } else {
                    ui.label(RichText::new(format!("No Camera Position")).size(16.));
                }
            };
        });

        ui.separator();

        let steps = [0.01, 0.1, 0.5, 1., 5.];

        let Some((x, y, z)) = klipper.get_position() else {
            ui.label("No position");
            return;
        };

        let button_width = 50.;

        StripBuilder::new(ui)
            .sizes(egui_extras::Size::exact(100.), 2)
            .vertical(|mut strip| {
                strip.strip(|builder| {
                    builder
                        // .sizes(egui_extras::Size::relative(0.1), steps.len())
                        .sizes(egui_extras::Size::exact(button_width), steps.len())
                        .size(egui_extras::Size::exact(100.))
                        .sizes(egui_extras::Size::exact(button_width), steps.len())
                        // .sizes(egui_extras::Size::relative(0.1), steps.len())
                        .cell_layout(egui::Layout::default().with_cross_align(egui::Align::Center))
                        .horizontal(|mut strip| {
                            let mut strip = self.button_range(strip, 0, &steps, true);

                            strip.cell(|ui| {
                                ui.label(RichText::new(format!("X: {:.2}", x)).size(20.));
                            });

                            let mut strip = self.button_range(strip, 0, &steps, false);
                        });
                });

                strip.strip(|builder| {
                    builder
                        // .sizes(egui_extras::Size::relative(0.1), steps.len())
                        .sizes(egui_extras::Size::exact(button_width), steps.len())
                        .size(egui_extras::Size::exact(100.))
                        // .sizes(egui_extras::Size::relative(0.1), steps.len())
                        .sizes(egui_extras::Size::exact(button_width), steps.len())
                        .cell_layout(egui::Layout::default().with_cross_align(egui::Align::Center))
                        .horizontal(|mut strip| {
                            let mut strip = self.button_range(strip, 1, &steps, true);

                            strip.cell(|ui| {
                                ui.label(RichText::new(format!("Y: {:.2}", y)).size(20.));
                            });

                            let mut strip = self.button_range(strip, 1, &steps, false);
                        });
                });
            });

        // ui.horizontal(|ui| {
        //     if let Some(pos) = klipper.get_position() {
        //         ui.label(format!("X: {:.3}", pos.0));
        //         ui.label(format!("Y: {:.3}", pos.1));
        //         ui.label(format!("Z: {:.3}", pos.2));
        //     } else {
        //         ui.label("No position");
        //     }
        // });

        //
    }

    fn button_range<'a, 'b>(
        &mut self,
        mut strip: egui_extras::Strip<'a, 'b>,
        axis: usize,
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
                    format!("{:.2}", -step)
                } else {
                    format!("+{:.2}", step)
                };
                let button = egui::Button::new(RichText::new(text).size(16.0))
                    .fill(egui::Color32::from_rgb(50, 158, 244))
                    .min_size(egui::vec2(60., 40.));

                // let layout = egui::Layout::default().with_cross_align(egui::Align::Center);

                // let button = ui.with_layout(layout, |ui| ui.add(button));

                if ui.add(button).clicked() {
                    eprintln!("Clicked on X: {}", -step);

                    let klipper = self.klipper.as_mut().unwrap();

                    let step = if neg { -step } else { step };
                    if let Err(e) = klipper.move_axis(axis, step, step.abs() < 1.0) {
                        self.errors.push(format!("Failed to move X: {}", e));
                    }
                }
            });
        }

        strip
    }

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
            let Some(klipper) = self.klipper.as_mut() else {
                ui.label("No Klipper connection");
                return;
            };

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
                    if let Err(e) = klipper.move_camera(cam_pos) {
                        self.errors.push(format!("Failed to move camera: {}", e));
                    }

                    //
                }
            }
        });
        //
    }

    fn webcam(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                egui::Grid::new("Filter Controls").show(ui, |ui| {
                    if ui.button("Clear Running Average").clicked() {
                        self.running_average.clear();
                    }
                    ui.end_row();

                    let (confidence, result) = self.running_average.get_result();
                    // let confidence = self.running_average.calculate_confidence();
                    ui.label(format!("Confidence: {:.3}", confidence));
                    ui.end_row();
                    if let Some(result) = result {
                        ui.label(format!("Result: ({:.3}, {:.3})", result.0, result.1));
                    } else {
                        ui.label("Result: None");
                    }

                    ui.end_row();

                    ui.separator();
                    ui.end_row();

                    self.webcam_controls(ui);
                });
            });

            let texture = match &self.webcam_texture {
                Some(texture) => texture,
                None => {
                    let image = egui::ColorImage::new([1280, 800], egui::Color32::from_gray(220));

                    let texture =
                        ui.ctx()
                            .load_texture("camera_texture", image, Default::default());

                    self.webcam_texture = Some(texture.clone());

                    // crate::webcam::Webcam::spawn_thread(
                    //     ui.ctx().clone(),
                    //     texture.clone(),
                    //     0,
                    //     self.crosshair_circle_size.clone(),
                    // );

                    let (tx, rx) = crossbeam_channel::unbounded();

                    self.channel_to_ui = Some(rx);

                    crate::vision::spawn_locator_thread(
                        ui.ctx().clone(),
                        texture.clone(),
                        0,
                        tx,
                        self.webcam_settings_mutex.clone(),
                    );

                    &self.webcam_texture.as_ref().unwrap()
                }
            };

            let size = egui::Vec2::new(
                crate::webcam::Webcam::SIZE.0 as f32,
                crate::webcam::Webcam::SIZE.1 as f32,
                // 640., 480.,
            );
            // let size = size / 1.;
            let size = size / 2.;

            let img = egui::Image::from_texture((texture.id(), size))
                .fit_to_exact_size(size)
                .max_size(size)
                // .rounding(egui::Rounding::same(4.))
                .sense(egui::Sense::click());

            let resp = ui.add(img);
        });
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

    fn webcam_controls(&mut self, ui: &mut egui::Ui) {
        ui.label("Filter Step");
        let resp = ui.add(
            egui::Slider::new(
                &mut self.webcam_settings.filter_step,
                0..=WebcamSettings::NUM_FILTER_STEPS,
            )
            .integer(),
        );

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
                    self.webcam_settings.filter_step += 1;
                } else if delta.y < 0. {
                    self.webcam_settings.filter_step -= 1;
                }
            }
        }
        ui.end_row();

        // ui.label("Threshold");
        // ui.add(egui::Slider::new(
        //     &mut self.webcam_settings.threshold,
        //     0..=100,
        // ));
        // ui.end_row();

        ui.label("Use Adaptive Threshold");
        ui.checkbox(&mut self.webcam_settings.adaptive_threshold, "");
        ui.end_row();

        ui.label("Adaptive Threshold Block Size");
        let resp = ui.add(
            egui::DragValue::new(&mut self.webcam_settings.adaptive_threshold_block_size)
                .speed(1.0)
                .fixed_decimals(0),
            // egui::Slider::new(
            //     &mut self.webcam_settings.adaptive_threshold_block_size,
            //     1..=100,
            // )
            // // .step_by(2.0)
            // .custom_formatter(|n, _| format!("{}", n * 2. + 1.)),
        );
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
                    self.webcam_settings.adaptive_threshold_block_size += 2;
                } else if delta.y < 0. {
                    self.webcam_settings.adaptive_threshold_block_size -= 2;
                }
            }
        }
        ui.end_row();

        ui.label("Adaptive Threshold C");
        ui.add(
            egui::DragValue::new(&mut self.webcam_settings.adaptive_threshold_c)
                .speed(0.1)
                .fixed_decimals(1),
        );
        ui.end_row();

        ui.label("Blur Kernel Size");
        ui.add(
            egui::DragValue::new(&mut self.webcam_settings.blur_kernel_size)
                .speed(1.0)
                .fixed_decimals(0),
        );
        ui.end_row();

        ui.label("Blur Sigma");
        ui.add(
            egui::DragValue::new(&mut self.webcam_settings.blur_sigma)
                .speed(0.1)
                .fixed_decimals(1),
        );

        ui.end_row();

        ui.label("Draw Circle");
        ui.checkbox(&mut self.webcam_settings.draw_circle, "");

        ui.end_row();

        ui.label("Use Hough");
        ui.checkbox(&mut self.webcam_settings.use_hough, "");

        ui.end_row();

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

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // if cfg!(debug_assertions) && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        //     ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        // }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        if let Some(rx) = self.channel_to_ui.as_mut() {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    crate::vision::WebcamMessage::FoundNozzle(pos) => {
                        // debug!("Found nozzle: {:?}", pos);
                        // self.running_average.push_position((pos.0, pos.1));
                        self.running_average.add_frame(Some(pos));
                    }
                    crate::vision::WebcamMessage::NozzleNotFound => {
                        // debug!("Nozzle not found");
                        self.running_average.add_frame(None)
                    }
                }
            }
        }

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.selectable_value(&mut self.current_tab, Tab::Webcam, "Webcam");
            ui.selectable_value(&mut self.current_tab, Tab::Options, "Options");
        });

        match self.current_tab {
            Tab::Webcam => {
                // #[cfg(feature = "nope")]
                egui::SidePanel::right("rigth")
                    .resizable(false)
                    .default_width(400.)
                    .show(ctx, |ui| {
                        // let Some(offsets) = self.tool_offsets else {
                        //     ui.label("No tool offsets");
                        //     return;
                        // };

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

                        // ui.label("Errors");
                        // if ui.button("Clear").clicked() {
                        //     self.errors.clear();
                        // }
                        // ui.group(|ui| {
                        //     for error in &self.errors {
                        //         ui.label(error);
                        //     }
                        // })
                    });

                egui::TopBottomPanel::bottom("bottom")
                    .resizable(false)
                    .default_height(200.)
                    .show(ctx, |ui| {
                        self.offset_adjust(ui);
                    });

                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        self.webcam(ui);
                    });
                    ui.separator();

                    ui.vertical_centered(|ui| {
                        self.controls(ui);
                    });

                    //
                });
            }
            Tab::Options => {
                self.options(ctx);
            }
        }
    }
}
