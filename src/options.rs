use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::ui::{auto_offset::AutoOffsetSettings, ui_types::App};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Options {
    pub camera_index: String,
    pub printer_url: String,
    pub num_tools: usize,
    pub bounce_amount: f64,
    pub camera_size: (f64, f64),
    pub camera_scale: f64,

    #[serde(skip)]
    pub auto_offset_settings: AutoOffsetSettings,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            camera_index: "0".to_string(),
            printer_url: "".to_string(),
            num_tools: 4,
            bounce_amount: 0.5,
            camera_size: (1280., 800.),
            camera_scale: 0.5,
            auto_offset_settings: AutoOffsetSettings::default(),
        }
    }
}

impl App {
    pub fn options(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self._options(ui);
        });
    }

    fn _options(&mut self, ui: &mut egui::Ui) {
        egui::widgets::global_theme_preference_buttons(ui);

        // ui.horizontal(|ui| {
        //     ui.label("Camera Index:");
        //     ui.text_edit_singleline(&mut self.options.camera_index);
        // });

        ui.separator();

        ui.horizontal(|ui| {
            egui::ComboBox::new("Camera Format", "Camera Format")
                // .selected_text(self.selected_camera_format.as_ref().map_or("None".to_string(), |f| f))
                .show_ui(ui, |ui| {
                    if self.camera_formats.len() == 0 {
                        if let Err(e) = self
                            .channel_to_vision
                            .as_ref()
                            .unwrap()
                            .send(crate::vision::WebcamCommand::GetCameraFormats)
                        {
                            error!("Failed to send command to webcam thread: {}", e);
                        }
                    } else {
                        for format in &self.camera_formats {
                            ui.selectable_value(
                                &mut self.selected_camera_format,
                                Some(*format),
                                format.to_string(),
                            );
                        }
                    }

                    // ui.selectable_value(&mut self.options.camera_size, (1280., 800.), "1280x800");
                    // ui.selectable_value(&mut self.options.camera_size, (1920., 1080.), "1920x1080");
                    // ui.selectable_value(&mut self.options.camera_size, (3840., 2160.), "3840x2160");
                });
        });
    }
}
