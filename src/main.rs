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
pub mod klipper_protocol;
pub mod logging;
pub mod options;
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

    let mut avg = vision::running_average::CircleAggregator::default();

    let p = (50., 50., 10.);

    for _ in 0..50 {
        avg.add_frame(Some(p));
    }
    for i in 0..0 {
        avg.add_frame(None);
        if let Some((confidence, (c_x, c_y, c_r))) = avg.confidence() {
            eprintln!(
                "Confidence[{}]: {:.3}, ({:.4}, {:.4}, r = {:.1})",
                i, confidence, c_x, c_y, c_r
            );
        } else {
            eprintln!("No confidence");
        }
    }

    if let Some((confidence, (c_x, c_y, c_r))) = avg.confidence() {
        eprintln!(
            "Confidence: {:.3}, ({:.4}, {:.4}, r = {:.1})",
            confidence, c_x, c_y, c_r
        );
    } else {
        eprintln!("No confidence");
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
