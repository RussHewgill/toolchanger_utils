pub mod commands;

use std::sync::Arc;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui_inbox::UiInboxSender;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio::{net::TcpStream, sync::RwLock, time::Instant};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use url::Url;

use crate::{ui::ui_types::Axis, vision::WebcamMessage};

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
    GetToolOffsets,
    DisableMotors,
    WaitForMoves,
    Dwell(u32),
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
}

pub struct KlipperConn {
    url: String,
    ws_write: SplitSink<
        WebSocketStream<MaybeTlsStream<TcpStream>>,
        tokio_tungstenite::tungstenite::Message,
    >,
    // ws_read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    // current_status: KlipperStatus,
    current_status: Arc<RwLock<KlipperStatus>>,
    inbox: UiInboxSender<KlipperMessage>,
    // inbox_position: UiInboxSender<(f64, f64, f64)>,
    channel_from_ui: tokio::sync::mpsc::Receiver<KlipperCommand>,
    id: usize,
}

#[derive(Clone, Debug)]
pub struct KlipperStatus {
    pub last_position_update: Instant,
    pub absolute_coordinates: bool,
    pub position: Option<(f64, f64, f64)>,
    // pub active_tool: Option<u32>,
    pub homed_axes: (bool, bool, bool),
    pub vars: Option<(Instant, serde_json::Value)>,
    pub resolution: f64,
    pub motors_enabled: (bool, bool, bool),
}

impl Default for KlipperStatus {
    fn default() -> Self {
        KlipperStatus {
            last_position_update: Instant::now(),
            absolute_coordinates: true,
            position: None,
            // active_tool: None,
            homed_axes: (false, false, false),
            vars: None,
            resolution: 0.0,
            motors_enabled: (false, false, false),
        }
    }
}

impl KlipperStatus {
    fn _update_pos(
        &mut self,
        sender: &UiInboxSender<KlipperMessage>,
        pos: &serde_json::Value,
    ) -> Result<()> {
        match (pos[0].as_f64(), pos[1].as_f64(), pos[2].as_f64()) {
            (Some(x), Some(y), Some(z)) => {
                let pos = (x, y, z);
                sender
                    .send(KlipperMessage::Position(pos))
                    .map_err(|e| anyhow!("Failed to send position message: {:?}", e))?;
                self.position = Some(pos);
                self.last_position_update = Instant::now();
            }
            _ => {
                error!("Invalid gcode_position: {:?}", pos);
            }
        }

        Ok(())
    }

    fn _update_resolution(&mut self, stepper_x: &serde_json::Value) -> Result<()> {
        let rot_dist = stepper_x["rotation_distance"]
            .as_str()
            .ok_or(anyhow!("Failed to parse rotation_distance"))?
            .parse::<f64>()?;
        let microsteps = stepper_x["microsteps"]
            .as_str()
            .ok_or(anyhow!("Failed to parse microsteps"))?
            .parse::<f64>()?;
        let steps_per_rot = stepper_x["full_steps_per_rotation"]
            .as_str()
            .ok_or(anyhow!("Failed to parse full_steps_per_rotation"))?
            .parse::<f64>()?;

        self.resolution = rot_dist / (microsteps * steps_per_rot);

        Ok(())
    }

    fn update(
        &mut self,
        sender: &UiInboxSender<KlipperMessage>,
        json: &serde_json::Value,
    ) -> Result<()> {
        // debug!("updating status");

        if let Some(stepper_x) = json.pointer("/result/status/configfile/config/stepper_x") {
            self._update_resolution(&stepper_x)?;
        }

        if let Some(pos) = json.pointer("/result/status/gcode_move/gcode_position") {
            self._update_pos(sender, pos)?;
        }

        if let Some(pos) = json.pointer("/result/status/toolhead/position") {
            self._update_pos(sender, pos)?;
        }

        let steppers = json.pointer("/result/status/stepper_enable/steppers");
        let steppers = json
            .pointer("/params/0/stepper_enable/steppers")
            .or(steppers);

        if let Some(steppers) = steppers {
            if let Some(x) = steppers.get("stepper_x").and_then(|v| v.as_bool()) {
                // debug!("stepper_x: {}", x);
                self.motors_enabled.0 = x;
            }
            if let Some(y) = steppers.get("stepper_y").and_then(|v| v.as_bool()) {
                // debug!("stepper_y: {}", y);
                self.motors_enabled.1 = y;
            }
            if let Some(z) = steppers.get("stepper_z").and_then(|v| v.as_bool()) {
                // debug!("stepper_z: {}", z);
                self.motors_enabled.2 = z;
            }
        }

        if let Some(vars) = json.pointer("/result/status/save_variables/variables") {
            self.vars = Some((Instant::now(), vars.clone()));
        }

        let Some(data) = json.pointer("/params/0/toolhead") else {
            // bail!("Failed to get toolhead data");
            return Ok(());
        };

        // debug!("updating status");
        if let Some(pos) = data.get("position") {
            self._update_pos(sender, pos)?;
        }

        if let Some(axes) = data.get("homed_axes").and_then(|v| v.as_str()) {
            let prev_axes = self.homed_axes;
            match axes {
                "" => self.homed_axes = (false, false, false),
                "xy" => self.homed_axes = (true, true, false),
                "xyz" => self.homed_axes = (true, true, true),
                _ => {
                    todo!("homed_axes: {:?}", axes);
                }
            }
            if self.homed_axes != prev_axes {
                sender
                    .send(KlipperMessage::AxesHomed(self.homed_axes))
                    .map_err(|e| anyhow!("Failed to send axes homed message: {:?}", e))?;
            }
        }

        if let Some(abs) = data
            .pointer("/params/0/gcode_move/absolute_coordinates")
            .and_then(|v| v.as_bool())
        {
            self.absolute_coordinates = abs;
        }

        Ok(())
    }
}

impl KlipperConn {
    pub async fn new(
        url: Url,
        inbox: UiInboxSender<KlipperMessage>,
        // inbox_position: UiInboxSender<(f64, f64, f64)>,
        rx: tokio::sync::mpsc::Receiver<KlipperCommand>,
        tx_status: tokio::sync::oneshot::Sender<Arc<RwLock<KlipperStatus>>>,
    ) -> Result<Self> {
        let url = format!("ws://{}:7125/websocket", url.host_str().unwrap());

        let (ws_stream, _) = connect_async(&url).await?;
        debug!("Connected to {}", &url);

        let (mut ws_write, mut ws_read) = ws_stream.split();

        // let (tx, rx) = crossbeam_channel::bounded(1);

        let current_status = Arc::new(RwLock::new(KlipperStatus::default()));

        let status2 = current_status.clone();
        let inbox2 = inbox.clone();
        tokio::spawn(Self::listener(status2, inbox2, ws_read));

        tx_status.send(current_status.clone()).unwrap_or_else(|e| {
            error!("Failed to send status: {:?}", e);
        });

        let mut out = KlipperConn {
            url,
            ws_write,
            // ws_read,
            current_status,
            inbox,
            channel_from_ui: rx,
            id: 1,
        };

        out.init().await?;

        Ok(out)
    }

    async fn init(&mut self) -> Result<()> {
        self.subscribe_to_defaults().await?;

        self.query_object("configfile")
            .await
            .map_err(|e| anyhow!("Failed to query object: {:?}", e))?;

        self.query_object("stepper_enable")
            .await
            .map_err(|e| anyhow!("Failed to query object: {:?}", e))?;

        Ok(())
    }

    async fn listener(
        status: Arc<RwLock<KlipperStatus>>,
        inbox: UiInboxSender<KlipperMessage>,
        mut ws_read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ) {
        debug!("Listening for messages");
        loop {
            // debug!("Listening for messages: Looping");
            let Some(msg) = ws_read.next().await else {
                warn!("WebSocket closed");
                return;
            };

            match msg {
                Ok(msg) => {
                    // debug!("handling msg");
                    Self::handle_message(&status, &inbox, msg)
                        .await
                        .unwrap_or_else(|e| {
                            error!("Failed to handle message: {}", e);
                        });
                }
                Err(e) => {
                    error!("Error receiving message: {}", e);
                    break;
                }
            }
        }
    }

    pub fn get_id(&mut self) -> usize {
        let id = self.id;
        self.id += 1;
        id
    }

    pub async fn subscribe_to_defaults(&mut self) -> Result<()> {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "printer.objects.subscribe",
            "params": {
                "objects": {
                    "gcode_move": ["absolute_coordinates"],
                    "toolhead": ["position", "status", "homed_axes"],
                    // "motion_report": null,
                    // "idle_timeout": null,
                    "stepper_enable": null,
                }
            },
            "id": self.get_id(),
        })
        .to_string();

        self.ws_write
            .send(tokio_tungstenite::tungstenite::Message::Text(msg.into()))
            .await?;
        Ok(())
    }

    pub async fn query_object(&mut self, object: &str) -> Result<()> {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "printer.objects.query",
            "params": {
                "objects": {
                    object: null,
                }
            },
            "id": self.get_id(),
        })
        .to_string();
        self.ws_write
            .send(tokio_tungstenite::tungstenite::Message::Text(msg.into()))
            .await?;
        Ok(())
    }

    pub async fn list_objects(&mut self) -> Result<()> {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "printer.objects.list",
            "id": self.get_id(),
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
            // debug!("looping");

            tokio::select! {
                // Some(Ok(msg)) = self.ws_read.next() => {
                //     self.handle_message(msg).unwrap();
                // }
                cmd = self.channel_from_ui.recv() => {
                    match cmd {
                        None => {
                            debug!("Channel closed");
                            return Ok(());
                        }
                        Some(cmd) => {
                            self.handle_command(cmd).await.unwrap();
                        }
                    }
                }
            };
        }
    }
}

impl KlipperConn {
    async fn handle_command(&mut self, cmd: KlipperCommand) -> Result<()> {
        match cmd {
            KlipperCommand::MoveToPosition(pos, bounce) => self.move_to_position(pos, bounce).await,
            KlipperCommand::MoveAxisRelative(axis, amount, bounce) => {
                self.move_axis_relative(axis, amount, bounce).await
            }
            KlipperCommand::HomeXY => self.home_xy().await,
            KlipperCommand::HomeAll => self.home_all().await,
            KlipperCommand::GetPosition(tx) => {
                tx.send(self.get_position().await.ok()).unwrap();
                Ok(())
            }
            KlipperCommand::PickTool(tool) => self.pick_tool(tool).await,
            KlipperCommand::DropTool => self.dropoff_tool().await,
            KlipperCommand::AdjustToolOffset(_, axis, _) => todo!(),
            KlipperCommand::GetToolOffsets => self.get_offsets().await,
            KlipperCommand::DisableMotors => self.disable_motors().await,
            KlipperCommand::WaitForMoves => self.wait_for_moves().await,
            KlipperCommand::Dwell(ms) => self.dwell(ms).await,
        }
    }

    // #[cfg(feature = "nope")]
    async fn handle_message(
        status: &RwLock<KlipperStatus>,
        inbox: &UiInboxSender<KlipperMessage>,
        msg: tokio_tungstenite::tungstenite::Message,
    ) -> Result<()> {
        let msg1 = msg.into_text().unwrap();
        let json = serde_json::from_str(msg1.as_str());

        let json: serde_json::Value = match json {
            Ok(json) => json,
            Err(e) => {
                trace!("Failed to parse JSON: {}\n{}", msg1, e);
                return Ok(());
            }
        };

        let method = json["method"].as_str().unwrap_or("");

        // debug!("Received message: {}", method);

        if method == "notify_proc_stat_update" {
            // debug!("Received: {}", serde_json::to_string_pretty(&json).unwrap());
            // debug!("got notify_proc_stat_update");

            if let Some(params) = json.pointer("params/0") {
                if params.get("stepper_enabled").is_some() {
                    trace!("got msg: {}", serde_json::to_string_pretty(&json).unwrap());
                }
            }

            // return Ok(());
        } else {
            trace!("got msg: {}", serde_json::to_string_pretty(&json).unwrap());
        }

        if let Err(e) = status.write().await.update(&inbox, &json) {
            error!("Failed to update status: {}", e);
        }

        // if method == "notify_status_update" {
        //     // debug!("updating");
        // } else if method != "" {
        //     debug!("method: {}", method);
        // }

        Ok(())
    }

    #[cfg(feature = "nope")]
    fn handle_message(&mut self, msg: tokio_tungstenite::tungstenite::Message) -> Result<()> {
        let msg = msg.into_text().unwrap();
        let json = serde_json::from_str(msg.as_str());

        let json: serde_json::Value = match json {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to parse JSON: {}\n{}", msg, e);
                return Ok(());
            }
        };

        // check if it's a response to a subscription request
        if let Some(m) = json.get("result").and_then(|v| v.get("status")) {
            debug!("Received result: {}", m);
            self.current_status.update(&self.inbox_position, &m);
        }

        let method = json["method"].as_str().unwrap_or("");

        // debug!("Received message: {}", method);

        if method != "notify_proc_stat_update" {
            // debug!("Received: {}", serde_json::to_string_pretty(&json).unwrap());
        }

        if method == "notify_status_update" {
            self.current_status.update(&self.inbox_position, &json);
        }

        // debug!("Received: {}", serde_json::to_string_pretty(&json)?);

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
