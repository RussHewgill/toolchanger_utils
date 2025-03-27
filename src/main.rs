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

    /// get all files in test_images/
    let mut paths = std::fs::read_dir("test_images").unwrap().enumerate();

    // 10 images per center
    let centers = [
        (944., 450.), // 0-9
        (947., 545.), // 10-19
        (370., 181.), // 20-29
        (635., 400.), // 30-39
    ];

    let mut detectors = vision::blob_detection::BlobDetectors::new().unwrap();

    let settings = {
        let mut settings = vision::VisionSettings::default();

        settings.adaptive_threshold = false;
        settings.threshold_block_size = 3;

        settings.blur_kernel_size = 7;
        settings.blur_sigma = 6.0;

        settings
    };

    let mut errors: Vec<Vec<(f64, f64)>> = vec![];

    for n in 0..centers.len() {
        let mut errors_i = vec![];

        for i in 0..10 {
            let path = paths.next().unwrap().1.unwrap().path();
            let mut img = image::ImageReader::open(path)?
                .decode()?
                .as_rgb8()
                .unwrap()
                .clone();

            let (_, pos) =
                vision::locate_nozzle::locate_nozzle(&mut img, &settings, &mut detectors)?;

            if let Some(pos) = pos {
                let error_x = (centers[n].0 - pos.0).abs();
                let error_y = (centers[n].1 - pos.1).abs();

                // debug!("Path {}: ({:.1}, {:.1})", i, pos.0, pos.1);
                // debug!("Error: ({:.1}, {:.1})", error_x, error_y);

                errors_i.push((error_x, error_y));
            } else {
                // debug!("No position found");
            }
        }

        errors.push(errors_i);
    }

    for (i, c) in centers.iter().enumerate() {
        eprintln!("Center: ({:.1}, {:.1})", c.0, c.1);

        let mut error_x = 0.;
        let mut error_y = 0.;

        let errors = &errors[i];

        if errors.len() == 0 {
            eprintln!("    No positions found");
            continue;
        }

        for e in errors {
            error_x += e.0;
            error_y += e.1;
        }

        error_x /= errors.len() as f64;
        error_y /= errors.len() as f64;

        eprintln!("    Average error: ({:.1}, {:.1})", error_x, error_y);
    }

    #[cfg(feature = "nope")]
    for (i, path) in paths.enumerate() {
        // read to ImageBuffer
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
