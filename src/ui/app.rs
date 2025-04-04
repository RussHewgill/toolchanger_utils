use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use super::ui_types::{App, Tab};

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

        // let mut webcam_settings = out.webcam_settings_mutex.lock().unwrap();
        // *webcam_settings = out.vision_settings;
        // drop(webcam_settings);

        // if let Err(e) = crate::appconfig::read_options_from_file("config.toml", &mut out.options) {
        //     error!("Failed to read options from file: {}", e);
        // }

        // if let Ok(data) = crate::saved_data::SavedData::load_from_file("saved_data.toml") {
        //     out.camera_pos = Some(data.camera_position);
        // }

        out
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        if !self.klipper_started {
            //
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
                egui::SidePanel::right("right")
                    .resizable(false)
                    .default_width(400.)
                    .show(ctx, |ui| {
                        //
                    });

                egui::CentralPanel::default().show(ctx, |ui| {
                    //
                });
            }
            Tab::Options => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    //
                });
            }
        }

        //
    }
}
