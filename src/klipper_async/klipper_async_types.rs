use std::sync::Arc;

use egui_inbox::UiInboxSender;
use futures_util::stream::SplitSink;
use tokio::{net::TcpStream, sync::RwLock, time::Instant};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::ui::ui_types::Axis;

#[derive(Debug)]
pub enum KlipperCommand {
    // MovePosition((f64, f64), Option<f64>),
    MoveToPosition((f64, f64, f64), Option<f64>),
    // MovePositionRelative((f64, f64), Option<f64>),
    MoveAxisRelative(Axis, f64, Option<f64>),
    HomeXY,
    HomeAll,
    GetPosition(tokio::sync::oneshot::Sender<Option<(f64, f64, f64)>>),
    PickTool(u32),
    DropTool,
    AdjustToolOffset(u32, Axis, f64),
    SetToolOffset(u32, Axis, f64),
    GetToolOffsets,
    DisableMotors,
    WaitForMoves,
    Dwell(u32),
    FetchPosition,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum KlipperMessage {
    Position((f64, f64, f64)),
    // MotorsDisabled,
    AxesHomed((bool, bool, bool)),
    // CameraPosition((f64, f64)),
    // ZHeight(f64),
    // ZHeightStale,
    KlipperError(String),
    ToolOffsets(Vec<(f64, f64, f64)>),
    HomingOriginChanged((f64, f64, f64)),
}

pub struct KlipperConn {
    pub(super) url: String,
    pub(super) ws_write: SplitSink<
        WebSocketStream<MaybeTlsStream<TcpStream>>,
        tokio_tungstenite::tungstenite::Message,
    >,
    // ws_read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    // current_status: KlipperStatus,
    pub(super) current_status: Arc<RwLock<KlipperStatus>>,
    pub(super) inbox: UiInboxSender<KlipperMessage>,
    // inbox_position: UiInboxSender<(f64, f64, f64)>,
    pub(super) channel_from_ui: tokio::sync::mpsc::Receiver<KlipperCommand>,
    pub(super) id: usize,
}

#[derive(Clone, Debug)]
pub struct KlipperStatus {
    pub last_position_update: Instant,
    pub absolute_coordinates: bool,
    pub position: Option<(f64, f64, f64)>,
    pub gcode_position: Option<(f64, f64, f64)>,
    // pub active_tool: Option<u32>,
    pub homed_axes: (bool, bool, bool),
    pub vars: Option<(Instant, serde_json::Value)>,
    pub resolution: f64,
    pub motors_enabled: (bool, bool, bool),
    pub homing_origin: (f64, f64, f64),
}

impl Default for KlipperStatus {
    fn default() -> Self {
        KlipperStatus {
            last_position_update: Instant::now(),
            absolute_coordinates: true,
            position: None,
            gcode_position: None,
            // active_tool: None,
            homed_axes: (false, false, false),
            vars: None,
            resolution: 0.0,
            motors_enabled: (false, false, false),
            homing_origin: (0.0, 0.0, 0.0),
        }
    }
}
