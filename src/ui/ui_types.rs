use std::sync::{Arc, Mutex};

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct App {
    #[serde(skip)]
    pub klipper: Option<crate::klipper_protocol::KlipperProtocol>,

    #[serde(skip)]
    pub errors: Vec<String>,

    #[serde(skip)]
    pub tried_startup_connection: bool,

    #[serde(skip)]
    pub tool_offsets: Vec<(f64, f64, f64)>,

    pub camera_pos: Option<(f64, f64)>,

    #[serde(skip)]
    pub active_tool: Option<usize>,

    #[serde(skip)]
    pub webcam_texture: Option<egui::TextureHandle>,

    pub crosshair_circle_size: std::sync::Arc<std::sync::atomic::AtomicU32>,

    #[serde(skip)]
    pub offset_axis: Axis,

    #[serde(skip)]
    pub offset_value: f64,

    #[serde(skip)]
    pub current_tab: Tab,

    #[serde(skip)]
    pub webcam_settings_prev: crate::vision::WebcamSettings,

    pub webcam_settings: crate::vision::WebcamSettings,

    #[serde(skip)]
    pub webcam_settings_mutex: Arc<Mutex<crate::vision::WebcamSettings>>,

    pub options: crate::options::Options,

    #[serde(skip)]
    // pub running_average: crate::vision::RunningAverage,
    pub running_average: crate::vision::vision_types::CircleAggregator,

    #[serde(skip)]
    pub channel_to_ui: Option<crossbeam_channel::Receiver<crate::vision::WebcamMessage>>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Tab {
    Webcam,
    Options,
}

impl Default for Tab {
    fn default() -> Self {
        Tab::Webcam
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Default for Axis {
    fn default() -> Self {
        Axis::X
    }
}
