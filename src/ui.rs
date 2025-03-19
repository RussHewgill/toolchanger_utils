use anyhow::{anyhow, bail, ensure, Context, Result};
use egui::RichText;
use egui_extras::StripBuilder;
use tracing::{debug, error, info, trace, warn};

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct App {
    #[serde(skip)]
    pub klipper: Option<crate::klipper_protocol::KlipperProtocol>,

    #[serde(skip)]
    pub errors: Vec<String>,

    #[serde(skip)]
    pub tried_startup_connection: bool,

    pub cxc_pos: Option<(f64, f64)>,
}

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
            }

            return;
        };

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

        ui.horizontal(|ui| {
            if ui.button(RichText::new("Dropoff Tool").size(16.)).clicked() {
                if let Err(e) = klipper.dropoff_tool() {
                    self.errors.push(format!("Failed to dropoff tool: {}", e));
                }
            }

            for t in 0..4 {
                if ui
                    .button(RichText::new(format!("T{}", t)).size(16.))
                    .clicked()
                {
                    if let Err(e) = klipper.pick_tool(t) {
                        self.errors
                            .push(format!("Failed to pick tool {}: {}", t, e));
                    }
                }
            }
            //
        });

        ui.separator();

        ui.horizontal(|ui| {
            if let Some((x, y, z)) = klipper.get_position() {
                if ui
                    .add(
                        egui::Button::new(RichText::new("Save CXC Position").size(16.))
                            .fill(egui::Color32::from_rgb(251, 149, 20)),
                    )
                    .clicked()
                {
                    self.cxc_pos = Some((x, y));
                    // klipper.set_cxc_pos(x, y);
                }

                if ui
                    .add(
                        egui::Button::new(RichText::new("Move to CXC").size(16.))
                            .fill(egui::Color32::from_rgb(50, 158, 244)),
                    )
                    .clicked()
                {
                    if let Some(pos) = self.cxc_pos {
                        if let Err(e) = klipper.move_cxc(pos) {
                            self.errors.push(format!("Failed to move to CXC: {}", e));
                        }
                    } else {
                        self.errors.push("No CXC position saved".to_string());
                    }
                }

                if let Some((cxc_x, cxc_y)) = self.cxc_pos {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("CXC Position: ({:.2}, {:.2})", cxc_x, cxc_y))
                                .size(16.),
                        );

                        let x = x - cxc_x;
                        let y = y - cxc_y;
                        ui.label(
                            RichText::new(format!("Diff from CXC: ({:.3}, {:.3})", x, y)).size(16.),
                        );
                    });
                } else {
                    ui.label(RichText::new(format!("No CXC Position")).size(16.));
                }
            };
        });

        ui.separator();

        let steps = [0.02, 0.1, 0.5, 1., 5.];

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
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // #[cfg(feature = "nope")]
        egui::TopBottomPanel::bottom("errors")
            // .resizable(false)
            // .default_width(600.)
            .show(ctx, |ui| {
                ui.label("Errors");
                if ui.button("Clear").clicked() {
                    self.errors.clear();
                }
                ui.group(|ui| {
                    for error in &self.errors {
                        ui.label(error);
                    }
                })
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.controls(ui);
            //
        });
    }
}
