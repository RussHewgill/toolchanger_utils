use anyhow::{anyhow, bail, ensure, Context, Result};
use opencv::{highgui, imgproc};
use tracing::{debug, error, info, trace, warn};

use std::collections::HashMap;

use crate::{ui::data_labeling::SavedTargets, vision::WebcamSettings};

pub fn main_tests() -> Result<()> {
    let results = [
        (284.028, 28.032),
        (284.031, 28.037),
        (284.029, 28.036),
        (284.030, 28.035),
    ];

    let mut xs = results.iter().map(|(x, _)| *x).collect::<Vec<_>>();
    let mut ys = results.iter().map(|(_, y)| *y).collect::<Vec<_>>();

    /// calculate median:
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let median_x = xs[xs.len() / 2];
    let median_y = ys[ys.len() / 2];

    debug!("Median: ({:.3}, {:.3})", median_x, median_y);

    /// use the median as the center point
    let xs = results
        .iter()
        .map(|(x, _)| x - median_x)
        .collect::<Vec<_>>();
    let ys = results
        .iter()
        .map(|(_, y)| y - median_y)
        .collect::<Vec<_>>();

    for i in 0..results.len() {
        debug!("{}: ({:.3}, {:.3})", i, xs[i], ys[i]);
    }

    Ok(())
}

#[cfg(feature = "nope")]
pub fn main_tests() -> Result<()> {
    let mut saved_targets = {
        let path = "test_images/saved_targets.toml";
        if let Ok(s) = std::fs::read_to_string(&path) {
            let saved_targets: SavedTargets = toml::from_str(&s)?;
            saved_targets
        } else {
            SavedTargets::default()
        }
    };

    let mut errors: HashMap<String, (f64, f64)> = HashMap::new();
    let mut misses: Vec<String> = vec![];

    let settings = WebcamSettings::default();

    let mut detectors = crate::vision::blob_detection::BlobDetectors::new()?;

    let prev_misses = {
        let mut prev_misses = std::collections::HashSet::new();
        prev_misses.insert("test_images/frame_0046.jpg".to_string());
        prev_misses.insert("test_images/frame_0045.jpg".to_string());
        prev_misses.insert("test_images/frame_0047.jpg".to_string());
        prev_misses.insert("test_images/frame_0051.jpg".to_string());
        prev_misses.insert("test_images/frame_0050.jpg".to_string());
        prev_misses.insert("test_images/frame_0052.jpg".to_string());
        prev_misses.insert("test_images/frame_0054.jpg".to_string());
        prev_misses.insert("test_images/frame_0053.jpg".to_string());
        prev_misses
    };

    // #[cfg(feature = "nope")]
    for (path, target) in saved_targets.targets.iter() {
        // let path = "test_images/frame_0000.jpg";
        // let target = (966.125, 431.0);

        if !prev_misses.contains(path) {
            continue;
        }

        let mut image = image::open(path)?.into_rgb8();

        let (w, h) = (image.width(), image.height());
        // debug!("Image: {} ({}x{})", path, w, h);

        let (mat, result) = match crate::vision::locate_nozzle::locate_nozzle(
            &mut image,
            &settings,
            &mut detectors,
        ) {
            Err(e) => {
                error!("Failed to locate nozzle in image {}: {}", path, e);
                continue;
            }
            Ok(result) => result,
        };

        // debug!("Result: {:?}", result);

        #[cfg(feature = "nope")]
        {
            // Create a clone of the OpenCV Mat for display
            let mut display_mat = mat.clone();

            // Draw the target point (ground truth)
            imgproc::circle(
                &mut display_mat,
                opencv::core::Point::new(target.0 as i32, target.1 as i32),
                5,
                opencv::core::Scalar::new(0.0, 255.0, 0.0, 0.0), // Green
                2,
                imgproc::LINE_8,
                0,
            )?;

            if let Some((x, y, radius)) = result {
                // Draw the detected point
                imgproc::circle(
                    &mut display_mat,
                    opencv::core::Point::new(x as i32, y as i32),
                    radius as i32,
                    opencv::core::Scalar::new(0.0, 0.0, 255.0, 0.0), // Red
                    2,
                    imgproc::LINE_8,
                    0,
                )?;

                let error_x = (target.0 - x).abs();
                let error_y = (target.1 - y).abs();
                errors.insert(path.to_string(), (error_x, error_y));

                if result.is_none() {
                    // Display error text
                    let error_text = format!("Error: ({:.1}, {:.1})", error_x, error_y);
                    imgproc::put_text(
                        &mut display_mat,
                        &error_text,
                        opencv::core::Point::new(10, 30),
                        imgproc::FONT_HERSHEY_SIMPLEX,
                        0.7,
                        opencv::core::Scalar::new(255.0, 255.0, 255.0, 0.0),
                        2,
                        imgproc::LINE_AA,
                        false,
                    )?;
                }
            } else {
                // Display "No detection" text
                imgproc::put_text(
                    &mut display_mat,
                    "No detection",
                    opencv::core::Point::new(10, 30),
                    imgproc::FONT_HERSHEY_SIMPLEX,
                    0.7,
                    opencv::core::Scalar::new(0.0, 0.0, 255.0, 0.0),
                    2,
                    imgproc::LINE_AA,
                    false,
                )?;
                misses.push(path.to_string());
            }

            // opencv::imgcodecs::imwrite(
            //     &format!("test.jpg"),
            //     &display_mat,
            //     &opencv::core::Vector::new(),
            // )
            // .unwrap();

            // // Display image in a window
            // let window_name = format!("Image: {}", path);
            // highgui::named_window(&window_name, highgui::WINDOW_NORMAL)?;
            // highgui::resize_window(&window_name, 1280, 800)?;

            // highgui::imshow(&window_name, &display_mat)?;

            // // Wait for key press (0 = wait indefinitely, or specify milliseconds)
            // // Use a small delay like 100ms to automatically proceed after showing each image
            // let key = highgui::wait_key(10_000)?;
            // if key == 27 {
            //     // ESC key to exit early
            //     break;
            // }
        }

        if let Some((x, y, radius)) = result {
            let error_x = (target.0 - x).abs();
            let error_y = (target.1 - y).abs();
            errors.insert(path.to_string(), (error_x, error_y));
        } else {
            // errors.insert(path.to_string(), (1e6, 1e6));
            // errors.insert(path.to_string(), (0.0, 0.0)); // Default error if no detection
            misses.push(path.to_string());
        }
    }

    let mut total_error = (0.0, 0.0);

    for (path, (error_x, error_y)) in errors.iter() {
        total_error.0 += error_x;
        total_error.1 += error_y;
    }

    let total_error = (
        total_error.0 / errors.len() as f64,
        total_error.1 / errors.len() as f64,
    );

    debug!("Total Error: {:.1}, {:.1}", total_error.0, total_error.1);
    debug!("Hits: {:?}", errors.len());
    debug!("Misses: {:?}", misses.len());

    for miss in misses.iter() {
        debug!("Miss: {}", miss);
    }

    //
    Ok(())
}
