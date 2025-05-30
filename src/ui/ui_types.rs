use std::sync::Arc;

use crate::vision::{
    blob_detection::BlobParams,
    preprocess::{PreprocessStep, PreprocessStepType},
};

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct App {
    // #[serde(skip)]
    // pub klipper: Option<crate::klipper_protocol::KlipperProtocol>,
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
    pub current_tab: Tab,

    #[serde(skip)]
    pub vision_settings_prev: crate::vision::VisionSettings,

    pub vision_settings: crate::vision::VisionSettings,

    #[serde(skip)]
    pub webcam_settings_mutex: Arc<std::sync::Mutex<crate::vision::VisionSettings>>,

    pub options: crate::ui::options::Options,

    #[serde(skip)]
    // pub running_average: crate::vision::RunningAverage,
    // pub running_average: crate::vision::vision_types::CircleAggregator,
    pub running_average: crate::vision::running_average::CircleAggregator,

    #[serde(skip)]
    pub channel_to_ui: Option<crossbeam_channel::Receiver<crate::vision::WebcamMessage>>,

    #[serde(skip)]
    pub channel_to_vision: Option<crossbeam_channel::Sender<crate::vision::WebcamCommand>>,

    #[serde(skip)]
    // pub auto_offset: Option<crate::ui::auto_offset::AutoOffset>,
    pub auto_offset: crate::ui::auto_offset_types::AutoOffset,

    #[serde(skip)]
    pub data_labeling: crate::ui::data_labeling::DataLabeling,

    // #[serde(skip)]
    // pub current_located_nozzle: Option<(f64, f64, f64)>,
    #[serde(skip)]
    pub camera_settings: crate::vision::CameraSettings,

    pub preprocess_add: PreprocessStepType,

    pub preprocess_pipeline: Vec<PreprocessStep>,

    #[serde(skip)]
    pub camera_formats: Vec<crate::vision::vision_types::CameraFormat>,
    #[serde(skip)]
    pub camera_formats_request_sent: bool,
    // #[serde(skip)]
    pub selected_camera_format: Option<crate::vision::CameraFormat>,

    #[serde(skip)]
    pub klipper_started: bool,

    #[serde(skip)]
    pub klipper_tx: Option<tokio::sync::mpsc::Sender<crate::klipper_async::KlipperCommand>>,
    #[serde(skip)]
    pub inbox: egui_inbox::UiInbox<crate::klipper_async::KlipperMessage>,

    #[serde(skip)]
    pub klipper_status: Option<Arc<tokio::sync::RwLock<crate::klipper_async::KlipperStatus>>>,

    #[serde(skip)]
    pub klipper_status_frame: Option<crate::klipper_async::KlipperStatus>,

    #[serde(skip)]
    /// for display only, not for sending to klipper
    pub last_position: (f64, f64, f64),

    #[serde(skip)]
    pub last_position_fetch: Option<std::time::Instant>,

    #[serde(skip)]
    pub blob_params: BlobParams,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Tab {
    Webcam,
    Options,
    // DataLabeling,
}

impl Default for Tab {
    fn default() -> Self {
        Tab::Webcam
        // Tab::DataLabeling
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl std::fmt::Display for Axis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl Axis {
    pub fn to_str(&self) -> &str {
        match self {
            Axis::X => "X",
            Axis::Y => "Y",
            Axis::Z => "Z",
        }
    }
}

impl Default for Axis {
    fn default() -> Self {
        Axis::X
    }
}
