use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui_inbox::UiInboxSender;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use url::Url;

use crate::{ui::ui_types::Axis, vision::WebcamMessage};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum KlipperCommand {
    MovePosition((f64, f64), Option<f64>),
    MovePositionRelative((f64, f64), Option<f64>),
    HomeXY,
    HomeAll,
    PickTool(u32),
    DropTool,
    AdjustToolOffset(u32, Axis, f64),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum KlipperMesssage {
    Position((f64, f64, f64)),
    MotorsDisabled,
    // CameraPosition((f64, f64)),
    // ZHeight(f64),
    // ZHeightStale,
    KlipperError(String),
}

struct KlipperConn {
    url: String,
    ws_write: SplitSink<
        WebSocketStream<MaybeTlsStream<TcpStream>>,
        tokio_tungstenite::tungstenite::Message,
    >,
    ws_read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    current_status: KlipperStatus,
    inbox: UiInboxSender<KlipperMesssage>,
    channel_from_ui: crossbeam_channel::Receiver<KlipperCommand>,
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KlipperStatus {
    pub motors_disabled: bool,
    pub speed: f64,
    pub absolute_coordinates: bool,
    pub homing_origin: (f64, f64, f64),
    pub gcode_position: Option<(f64, f64, f64)>,
    pub toolhead_position: Option<(f64, f64, f64)>,
    pub active_tool: Option<u32>,
}

impl KlipperConn {
    pub async fn new(
        url: Url,
        inbox: UiInboxSender<KlipperMesssage>,
    ) -> Result<(Self, crossbeam_channel::Sender<KlipperCommand>)> {
        let url = format!("ws://{}/websocket", url.host_str().unwrap());

        let (ws_stream, _) = connect_async(&url).await?;
        println!("Connected to {}", &url);

        let (mut ws_write, mut ws_read) = ws_stream.split();

        let (tx, rx) = crossbeam_channel::bounded(1);

        Ok((
            KlipperConn {
                url,
                ws_write,
                ws_read,
                current_status: KlipperStatus::default(),
                inbox,
                channel_from_ui: rx,
            },
            tx,
        ))
    }

    async fn subscribe_to_defaults(&mut self) -> Result<()> {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "printer.objects.subscribe",
            "params": {
                "objects": {
                    "gcode_move": null,
                    "toolhead": ["position", "status"]
                }
            },
            "id": 1
        })
        .to_string();

        self.ws_write
            .send(tokio_tungstenite::tungstenite::Message::Text(msg.into()))
            .await?;
        Ok(())
    }
}

/// main loop
impl KlipperConn {
    pub async fn run(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                Some(Ok(msg)) = self.ws_read.next() => {
                    self.handle_message(msg)?;
                }
            }
        }
    }
}

impl KlipperConn {
    fn handle_message(&mut self, msg: tokio_tungstenite::tungstenite::Message) -> Result<()> {
        let json = msg.into_text()?;
        let json: serde_json::Value = serde_json::from_str(json.as_str())?;

        debug!("Received: {}", serde_json::to_string_pretty(&json)?);

        Ok(())
    }
}

pub fn start_async_klipper_thread(
    ctx: egui::Context,
    channel_from_ui: &crossbeam_channel::Receiver<KlipperCommand>,
    mut handle: egui::TextureHandle,
) {
    std::thread::spawn(move || {
        //
    });
}
