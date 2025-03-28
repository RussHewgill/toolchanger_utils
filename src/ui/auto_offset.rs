use anyhow::{anyhow, bail, ensure, Context, Result};
use egui::RichText;
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

    check_repeatability: Option<usize>,
    repeatability: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AutoOffsetSettings {
    pub target_max_offset: f64,
    pub max_margin_of_error: f64,
    pub min_interval_between_moves: f64,
}

impl Default for AutoOffsetSettings {
    fn default() -> Self {
        AutoOffsetSettings {
            target_max_offset: 0.01,
            max_margin_of_error: 0.2,
            min_interval_between_moves: 2.0,
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
            check_repeatability: None,
            repeatability: Vec::new(),
        }
    }
}

impl AutoOffset {
    pub fn auto_offset_type(&self) -> AutoOffsetType {
        self.auto_offset_type
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
        unimplemented!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoOffsetType {
    None,
    SingleTool,
    AllTools,
    RepeatabilityTest,
}

impl App {
    pub fn auto_offset(&mut self, ui: &mut egui::Ui) {
        self._auto_offset(ui);

        match self.auto_offset.auto_offset_type {
            AutoOffsetType::None => self._auto_offset_none(ui),
            AutoOffsetType::SingleTool => todo!(),
            AutoOffsetType::AllTools => todo!(),
            AutoOffsetType::RepeatabilityTest => todo!(),
        }
    }

    fn _auto_offset(&mut self, ui: &mut egui::Ui) {
        let Some(pos) = self.get_position() else {
            ui.label("No position data available");
            return;
        };

        if (pos.0, pos.1) != self.auto_offset.prev_position {
            self.auto_offset.prev_position = (pos.0, pos.1);
            self.running_average.clear();
        }

        if let Some((confidence, (c_x, c_y, c_r))) = self.running_average.confidence() {
            ui.label(
                RichText::new(format!(
                    "Confidence: {:.3}, ({:.4}, {:.4}, r = {:.1})",
                    confidence, c_x, c_y, c_r
                ))
                .monospace(),
            );
        } else {
            ui.label(RichText::new("Confidence: None").monospace());
        }

        if let Some((x, y, r)) = self.running_average.current_guess() {
            ui.label(
                RichText::new(format!("Current Guess: ({:.4}, {:.4}, r = {:.1})", x, y, r))
                    .monospace(),
            );
        } else {
            ui.label(RichText::new("Current Guess: None").monospace());
        }

        // let Some((median, moe)) = self.running_average.calculate_margin_of_error() else {
        //     // debug!("Failed to calculate margin of error");
        //     return;
        // };

        // if moe.0 > AutoOffset::MAX_MARGIN_OF_ERROR && moe.1 > AutoOffset::MAX_MARGIN_OF_ERROR {
        //     // debug!("Margin of error is too high: {:?}", moe);
        //     return;
        // }

        //
    }

    fn _auto_offset_none(&mut self, ui: &mut egui::Ui) {
        ui.label("No auto offset selected");
    }
}
