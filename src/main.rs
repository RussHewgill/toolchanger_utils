#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_doc_comments)]
#![allow(unused_labels)]
#![allow(unexpected_cfgs)]
// #![windows_subsystem = "windows"]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod klipper_protocol;
pub mod logging;
pub mod ui;
pub mod webcam;

use std::collections::HashMap;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

// #[cfg(feature = "nope")]
fn main() -> eframe::Result<()> {
    use ui::App;

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
