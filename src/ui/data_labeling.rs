use std::collections::HashMap;

use super::ui_types::App;

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct DataLabeling {
    /// tool index, screen position
    pub target: Option<(usize, egui::Pos2)>,
    // pub target: Option<egui::Pos2>,
    pub num_screens: usize,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct SavedTargets {
    pub index: usize,
    pub targets: HashMap<String, (f64, f64)>,
}
