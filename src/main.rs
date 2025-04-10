#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_doc_comments)]
#![allow(unused_labels)]
#![allow(unexpected_cfgs)]
// // #![windows_subsystem = "windows"]
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod appconfig;
pub mod klipper_async;
pub mod klipper_protocol;
pub mod logging;
pub mod saved_data;
pub mod tests;
pub mod tuning;
pub mod ui;
pub mod vision;
// pub mod webcam;

use std::collections::HashMap;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

#[cfg(feature = "nope")]
fn main() -> opencv::Result<()> {
    logging::init_logs();
    debug!("Init");

    let path = "frame_centered.jpg";
    // let path = "frame_up.jpg";

    // vision::opencv_test()?;

    let pos = vision::locate_nozzle(&path)?;

    debug!("Position: {:?}", pos);

    Ok(())
}

#[cfg(feature = "nope")]
fn main() -> Result<()> {
    logging::init_logs();

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .danger_accept_invalid_certs(true)
        .build()
        .context("Failed to build HTTP client")?;

    let url = url::Url::parse("http://192.168.0.245").unwrap();

    let url = url.join("/printer/objects/query")?;
    // let url = url.join("/printer/objects/list")?;

    let map = serde_json::json!({
        "objects": {
            "configfile": null
            // "toolhead": null
        }
    });

    let res = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&map)
        .send()
        .context("Failed to send request")?;

    let json = res
        .json::<serde_json::Value>()
        .context("Failed to parse response")?;

    let stepper_x = json
        .pointer("/result/status/configfile/config/stepper_x")
        .unwrap();

    debug!(
        "Response: {}",
        serde_json::to_string_pretty(&stepper_x).unwrap()
    );

    let rot_dist = stepper_x["rotation_distance"]
        .as_str()
        .unwrap()
        .parse::<f64>()?;
    let microsteps = stepper_x["microsteps"].as_str().unwrap().parse::<f64>()?;
    let steps_per_rot = stepper_x["full_steps_per_rotation"]
        .as_str()
        .unwrap()
        .parse::<f64>()?;

    debug!("Rotational distance: {}", rot_dist);
    debug!("Microsteps: {}", microsteps);
    debug!("Steps per rotation: {}", steps_per_rot);

    let resolution = rot_dist / (microsteps * steps_per_rot);

    debug!("Resolution: {}", resolution);

    Ok(())
}

/// Async klipper tests
#[cfg(feature = "nope")]
#[tokio::main]
async fn main() -> Result<()> {
    logging::init_logs();
    // // let url = "ws://192.168.0.245:7125/klippysocket";
    // let url = "ws://192.168.0.245:7125/websocket";

    // let url = url::Url::parse("ws://192.168.0.245:7125/klippysocket")?;
    let url = url::Url::parse("ws://192.168.0.245:7125/websocket")?;

    let inbox: egui_inbox::UiInbox<klipper_async::KlipperMessage> = egui_inbox::UiInbox::new();

    let (tx, rx) = tokio::sync::mpsc::channel(1);

    // tx.send(klipper_async::KlipperCommand::MoveToPosition(
    //     (150., 150., 40.),
    //     Some(0.5),
    // ))
    // .await?;

    let (tx2, mut rx2) = tokio::sync::oneshot::channel();

    let mut klipper = klipper_async::KlipperConn::new(url, inbox.sender(), rx, tx2).await?;

    // klipper.subscribe_to_defaults().await?;

    // klipper.list_objects().await?;
    klipper.query_object("stepper_x").await?;

    klipper.run().await?;

    Ok(())
}

/// Async klipper tests
#[cfg(feature = "nope")]
#[tokio::main]
async fn main() -> Result<()> {
    logging::init_logs();

    use futures_util::{SinkExt, StreamExt};
    use tokio::io::AsyncBufReadExt;
    use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

    // let url = url::Url::parse("ws://192.168.0.245:7125/klippysocket")?;
    // let url = "ws://192.168.0.245:7125/klippysocket";
    let url = "ws://192.168.0.245:7125/websocket";

    let (ws_stream, _) = connect_async(url).await?;
    debug!("Connected to {}", url);

    // Split the WebSocket stream into write and read halves
    let (mut write, mut read) = ws_stream.split();
    debug!("Split WebSocket stream");

    let msg = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "printer.objects.subscribe",
        "params": {
            "objects": {
                // "gcode_move": null,
                // "toolhead": ["position", "status"]
                "stepper_enable": null,
            }
        },
        "id": 1
    })
    .to_string();

    if let Err(e) = write.send(Message::Text(msg.into())).await {
        eprintln!("Error sending message: {}", e);
    }

    // async fn send(
    //     mut write: futures_util::stream::SplitSink<
    //         tokio_tungstenite::WebSocketStream<
    //             tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    //         >,
    //         tokio_tungstenite::tungstenite::Message,
    //     >,
    // ) {
    //     let mut id = 2;
    //     debug!("Spawned send task");

    //     loop {
    //         debug!("Sending message");
    //         let msg = serde_json::json!({
    //             "jsonrpc": "2.0",
    //             "method": "printer.objects.query",
    //             "params": {
    //                 "objects": {
    //                     // "gcode_move": null,
    //                     // "toolhead": ["position", "status"]
    //                     "stepper_enable": null,
    //                 }
    //             },
    //             "id": id,
    //         })
    //         .to_string();

    //         id += 1;

    //         if let Err(e) = write.send(Message::Text(msg.into())).await {
    //             eprintln!("Error sending message: {}", e);
    //         }

    //         tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    //     }
    // }

    // debug!("Spawning send task");
    // tokio::spawn(send(write));
    // debug!("done");

    // Handle incoming messages
    while let Some(message) = read.next().await {
        match message {
            Ok(Message::Text(text)) => {
                // let msg1 = text.into_text().unwrap();
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(text.as_str()) {
                    //

                    let method = json["method"].as_str().unwrap_or("");

                    if method != "notify_proc_stat_update" {
                        debug!("Received: {}", serde_json::to_string_pretty(&json).unwrap());
                    }
                }
                // println!("Received: {}\n", text),
            }
            Ok(Message::Close(_)) => {
                println!("Connection closed");
                break;
            }
            Err(e) => {
                eprintln!("Error receiving message: {}", e);
                break;
            }
            _ => {} // Ignore other message types
        }
    }

    Ok(())
}

#[cfg(feature = "tests")]
fn main() -> Result<()> {
    logging::init_logs();

    tests::main_tests().unwrap();

    Ok(())
}

/// Main App
// #[cfg(feature = "nope")]
#[cfg(not(feature = "tests"))]
fn main() -> eframe::Result<()> {
    use ui::ui_types::App;

    logging::init_logs();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };
    eframe::run_native(
        "toolchanger_utils",
        native_options,
        Box::new(|cc| Ok(Box::new(ui::ui_types::App::new(cc)))),
    )
}

#[cfg(feature = "nope")]
fn main() -> Result<()> {
    logging::init_logs();

    // println!("Hello, world!");

    // let url = "http://192.168.0.245/server/jsonrpc";
    let url = "http://192.168.0.245";

    // let mut klipper = klipper_protocol::KlipperProtocol::new(url)?;

    // let vars = klipper.get_variables()?;

    // pretty print

    // let pretty_vars = serde_json::to_string_pretty(&vars)?;
    // println!("{}", pretty_vars);

    // klipper.get_position()?;

    // klipper.home_xy()?;

    // klipper.run_gcode("_CLIENT_LINEAR_MOVE X=1")?;

    let index = 0;

    {
        use nokhwa::{
            pixel_format::RgbFormat,
            utils::{RequestedFormat, RequestedFormatType},
        };

        let format =
        // RequestedFormat::<RgbFormat>::new(RequestedFormatType::AbsoluteHighestFrameRate);
        RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
        let mut camera =
            nokhwa::Camera::new(nokhwa::utils::CameraIndex::Index(index as u32), format).unwrap();

        let Ok(frame) = camera.frame() else {
            eprintln!("Failed to get frame");
            return Ok(());
        };

        let res = frame.resolution();

        debug!("Got frame: {}x{}", res.width(), res.height());

        //
    }

    Ok(())
}
