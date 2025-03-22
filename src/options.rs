use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::ui::ui_types::App;

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct Options {
    pub camera_index: String,
}

impl App {
    pub fn options(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self._options(ui);
        });
    }

    fn _options(&mut self, ui: &mut egui::Ui) {
        egui::widgets::global_theme_preference_buttons(ui);

        ui.horizontal(|ui| {
            ui.label("Camera Index:");
            ui.text_edit_singleline(&mut self.options.camera_index);
        });
    }
}
