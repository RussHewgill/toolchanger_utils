use egui::Slider;

use crate::vision::{CameraControl, VisionSettings, WebcamCommand};

use super::ui_types::App;

impl App {
    pub fn webcam_controls(&mut self, ui: &mut egui::Ui) {
        ui.label("Filter Step");
        let resp = ui.add(
            egui::Slider::new(
                &mut self.webcam_settings.filter_step,
                0..=VisionSettings::NUM_FILTER_STEPS,
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
                } else if delta.y < 0. && self.webcam_settings.filter_step > 0 {
                    self.webcam_settings.filter_step -= 1;
                }
            }
        }
        ui.end_row();

        ui.label("Pipeline");
        let resp = ui
            .add(egui::Slider::new(&mut self.webcam_settings.preprocess_pipeline, 0..=3).integer());
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
                    self.webcam_settings.preprocess_pipeline += 1;
                } else if delta.y < 0. && self.webcam_settings.preprocess_pipeline > 0 {
                    self.webcam_settings.preprocess_pipeline -= 1;
                }
            }
        }
        ui.end_row();

        // ui.label("Use Adaptive Threshold");
        // ui.checkbox(&mut self.webcam_settings.adaptive_threshold, "");
        // ui.end_row();

        ui.label("Threshold Block Size");
        let resp = ui.add(
            egui::DragValue::new(&mut self.webcam_settings.threshold_block_size)
                .speed(1.0)
                .fixed_decimals(0)
                .range(0..=255),
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
                    self.webcam_settings.threshold_block_size += 2;
                } else if delta.y < 0. {
                    self.webcam_settings.threshold_block_size -= 2;
                }
            }
        }
        ui.end_row();

        // ui.label("Adaptive Threshold C");
        // ui.add(
        //     egui::DragValue::new(&mut self.webcam_settings.adaptive_threshold_c)
        //         .speed(0.1)
        //         .fixed_decimals(1),
        // );
        // ui.end_row();

        ui.label("Threshold Type");
        let resp =
            ui.add(egui::Slider::new(&mut self.webcam_settings.threshold_type, 0..=2).integer());

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
                    self.webcam_settings.threshold_type += 1;
                } else if delta.y < 0. && self.webcam_settings.threshold_type > 0 {
                    self.webcam_settings.threshold_type -= 1;
                }
            }
        }
        ui.end_row();

        ui.label("Blur Kernel Size");
        let resp = ui.add(
            egui::DragValue::new(&mut self.webcam_settings.blur_kernel_size)
                .speed(1.0)
                .fixed_decimals(0),
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
                    self.webcam_settings.blur_kernel_size += 2;
                } else if delta.y < 0. {
                    self.webcam_settings.blur_kernel_size -= 2;
                }
            }
        }
        ui.end_row();

        ui.label("Blur Sigma");
        let resp = ui.add(
            egui::DragValue::new(&mut self.webcam_settings.blur_sigma)
                .speed(0.1)
                .fixed_decimals(1),
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
                    self.webcam_settings.blur_sigma += 0.25;
                } else if delta.y < 0. {
                    self.webcam_settings.blur_sigma -= 0.25;
                }
            }
        }
        ui.end_row();

        ui.label("Draw Circle");
        ui.checkbox(&mut self.webcam_settings.draw_circle, "");
        ui.end_row();

        ui.label("Use Hough");
        ui.checkbox(&mut self.webcam_settings.use_hough, "");
        ui.end_row();

        ui.label("Pixels to mm");
        ui.add(
            egui::DragValue::new(&mut self.webcam_settings.pixels_per_mm)
                .speed(0.1)
                .fixed_decimals(2),
        );
        ui.end_row();

        ui.label("Target Radius");
        ui.add(
            egui::DragValue::new(&mut self.webcam_settings.target_radius)
                .speed(1.)
                .fixed_decimals(0)
                .range(0.0..=150.0),
        );
        ui.end_row();

        ui.separator();
        ui.end_row();

        self.webcam_camera_controls(ui);
        ui.end_row();

        if self.webcam_settings != self.webcam_settings_prev {
            let mut settings = self.webcam_settings_mutex.lock().unwrap();
            *settings = self.webcam_settings;
            self.webcam_settings_prev = self.webcam_settings.clone();
        }
    }

    fn webcam_camera_controls(&mut self, ui: &mut egui::Ui) {
        ui.label("Brightness (-64 to 64)");
        if ui
            .add(Slider::new(&mut self.camera_settings.brightness, -64..=64))
            .changed()
        {
            self.channel_to_vision
                .as_ref()
                .unwrap()
                .send(WebcamCommand::SetCameraControl(CameraControl::Brightness(
                    self.camera_settings.brightness,
                )))
                .unwrap();
        }
        ui.end_row();

        ui.label("Contrast (0 to 100)");
        if ui
            .add(Slider::new(&mut self.camera_settings.contrast, 0..=100))
            .changed()
        {
            self.channel_to_vision
                .as_ref()
                .unwrap()
                .send(WebcamCommand::SetCameraControl(CameraControl::Contrast(
                    self.camera_settings.contrast,
                )))
                .unwrap();
        }
        ui.end_row();

        ui.label("Saturation (0 to 100)");
        if ui
            .add(Slider::new(&mut self.camera_settings.saturation, 0..=100))
            .changed()
        {
            self.channel_to_vision
                .as_ref()
                .unwrap()
                .send(WebcamCommand::SetCameraControl(CameraControl::Saturation(
                    self.camera_settings.saturation,
                )))
                .unwrap();
        }
        ui.end_row();

        ui.label("Sharpness (0 to 100)");
        if ui
            .add(Slider::new(&mut self.camera_settings.sharpness, 0..=100))
            .changed()
        {
            self.channel_to_vision
                .as_ref()
                .unwrap()
                .send(WebcamCommand::SetCameraControl(CameraControl::Sharpness(
                    self.camera_settings.sharpness,
                )))
                .unwrap();
        }
        ui.end_row();

        ui.label("Gamma (100 to 500)");
        if ui
            .add(Slider::new(&mut self.camera_settings.gamma, 100..=500))
            .changed()
        {
            self.channel_to_vision
                .as_ref()
                .unwrap()
                .send(WebcamCommand::SetCameraControl(CameraControl::Gamma(
                    self.camera_settings.gamma,
                )))
                .unwrap();
        }
        ui.end_row();

        // ui.label("White Balance (2800 to 6500)");
        // if ui
        //     .add(Slider::new(&mut self.camera_settings.white_balance, 2800..=6500).step_by(100.))
        //     .changed()
        // {
        //     self.channel_to_vision
        //         .as_ref()
        //         .unwrap()
        //         .send(WebcamCommand::SetCameraControl(
        //             CameraControl::WhiteBalance(self.camera_settings.white_balance),
        //         ))
        //         .unwrap();
        // }
        // ui.end_row();

        //
    }
}

pub fn draw_crosshair(radius: f32, img: &mut egui::ColorImage) {
    let width = img.width();
    let height = img.height();

    // Center coordinates
    let center_x = width / 2;
    let center_y = height / 2;

    // Crosshair size (length of each line from center)
    let line_length = width.min(height) / 20;

    // Crosshair color (bright green for visibility)
    let color = egui::Color32::from_rgb(255, 255, 0);

    for x in 0..width {
        img.pixels[x as usize + center_y * width as usize] = color;
    }

    for y in 0..height {
        img.pixels[center_x as usize + y * width as usize] = color;
    }

    // Draw circle outline
    // let radius = 100.;

    // Use Bresenham's circle algorithm
    let mut x = 0;
    let mut y = radius as i32;
    let mut d = 3 - 2 * (radius as i32);

    while y >= x {
        // Draw the eight octants
        draw_circle_points(img, center_x, center_y, x, y, color, width);

        if d > 0 {
            y -= 1;
            d += 4 * (x - y) + 10;
        } else {
            d += 4 * x + 6;
        }
        x += 1;
    }
}

// Helper function to draw the eight points of a circle at once
fn draw_circle_points(
    img: &mut egui::ColorImage,
    center_x: usize,
    center_y: usize,
    x: i32,
    y: i32,
    color: egui::Color32,
    width: usize,
) {
    let points = [
        (center_x + x as usize, center_y + y as usize),
        (center_x - x as usize, center_y + y as usize),
        (center_x + x as usize, center_y - y as usize),
        (center_x - x as usize, center_y - y as usize),
        (center_x + y as usize, center_y + x as usize),
        (center_x - y as usize, center_y + x as usize),
        (center_x + y as usize, center_y - x as usize),
        (center_x - y as usize, center_y - x as usize),
    ];

    for (px, py) in points {
        // Check bounds before drawing
        if px < img.width() && py < img.height() {
            img.pixels[px + py * width] = color;
        }
    }
}
