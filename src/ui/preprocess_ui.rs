use egui::{Frame, Vec2};

use crate::vision::preprocess::{PreprocessStep, PreprocessStepType};

use super::ui_types::App;

impl App {
    pub fn preprocess_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("Preprocess Add")
                .selected_text(self.preprocess_add.to_str())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.preprocess_add,
                        PreprocessStepType::ConvertGrayscale,
                        "Convert Grayscale",
                    );
                    ui.selectable_value(
                        &mut self.preprocess_add,
                        PreprocessStepType::ConvertLuma,
                        "Convert Luma",
                    );
                    ui.selectable_value(
                        &mut self.preprocess_add,
                        PreprocessStepType::GaussianBlur {
                            ksize: 7,
                            sigma: 6.0,
                        },
                        "Gaussian Blur",
                    );
                });

            if ui.button("Add").clicked() {
                self.preprocess_pipeline.push(PreprocessStep {
                    step: self.preprocess_add.clone(),
                    enabled: true,
                });
            }
        });

        ui.end_row();

        let frame = Frame::default().inner_margin(4.0);

        for (i, step) in self.preprocess_pipeline.iter_mut().enumerate() {
            Self::show_preprocess(ui, step);
        }

        #[cfg(feature = "nope")]
        let (_, dropped_payload) = ui.dnd_drop_zone::<usize, ()>(frame, |ui| {
            for (i, step) in self.preprocess_pipeline.iter_mut().enumerate() {
                let item_id = egui::Id::new(("preprocess_pipeline", i));

                // let response = ui
                //     .dnd_drag_source(item_id, i, |ui| {
                //         egui::containers::Resize::default()
                //             .fixed_size(Vec2::new(300., 200.))
                //             .show(ui, |ui| {
                //                 Self::show_preprocess(ui, step);
                //             });
                //     })
                //     .response;

                //
            }

            //
        });

        //
    }

    pub fn show_preprocess(ui: &mut egui::Ui, preprocess: &mut PreprocessStep) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut preprocess.enabled, "");
                ui.label(preprocess.step.to_str());
            });
            // if !preprocess.enabled {
            //     ui.disable();
            // }
        });
    }

    fn _show_preprocess(ui: &mut egui::Ui, step: &mut PreprocessStepType) {
        match step {
            PreprocessStepType::ConvertGrayscale => {}
            PreprocessStepType::ConvertLuma => {}
            PreprocessStepType::GaussianBlur {
                ref mut ksize,
                sigma,
            } => {
                ui.horizontal(|ui| {
                    ui.label("Gaussian Blur KSize:");
                    ui.add(
                        egui::DragValue::new(ksize)
                            .range(3..=100)
                            .fixed_decimals(0)
                            .speed(2),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Gaussian Blur Sigma:");
                    ui.add(egui::DragValue::new(sigma).range(-50.0..=50.0));
                });
            }
            PreprocessStepType::Threshold {
                threshold,
                threshold_type,
            } => todo!(),
            PreprocessStepType::AdaptiveThreshold => todo!(),
        }
    }
}
