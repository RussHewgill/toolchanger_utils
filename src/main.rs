#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_doc_comments)]
#![allow(unused_labels)]
#![allow(unexpected_cfgs)]
// // #![windows_subsystem = "windows"]
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod klipper_protocol;
pub mod logging;
pub mod options;
pub mod ui;
pub mod vision;
pub mod webcam;

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
fn main() {
    logging::init_logs();

    let poss = [
        (629.5, 404.5, 37.20000076293945),
        (734.5, 117.5, 33.79999923706055),
        (642.5, 403.5, 49.400001525878906),
        (640.5, 400.5, 48.29999923706055),
        (879.5, 655.5, 43.79999923706055),
        (634.5, 404.5, 41.599998474121094),
        (630.5, 412.5, 42.70000076293945),
        (834.5, 49.5, 36.099998474121094),
        (639.5, 411.5, 49.400001525878906),
        (631.5, 412.5, 42.70000076293945),
        (642.5, 403.5, 49.400001525878906),
        (637.5, 407.5, 45.0),
        (630.5, 412.5, 42.70000076293945),
        (637.5, 407.5, 45.0),
        (738.5, 116.5, 33.79999923706055),
        (626.5, 404.5, 33.79999923706055),
        (813.5, 184.5, 32.70000076293945),
        (988.5, 94.5, 49.400001525878906),
        (635.5, 405.5, 42.70000076293945),
        (640.5, 400.5, 48.29999923706055),
        (635.5, 405.5, 42.70000076293945),
    ];

    let mut agg = vision::vision_types::CircleAggregator::new(30, 50., 10.);

    for pos in poss.iter() {
        agg.add_frame(Some(*pos));
    }

    // let median = agg.calculate_median();

    let (confidence, result) = agg.get_result();

    debug!("Confidence: {}", confidence);
    debug!("Result: {:?}", result);

    //
}

/// Main App
// #[cfg(feature = "nope")]
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
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
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
