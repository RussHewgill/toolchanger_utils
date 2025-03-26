pub mod blob_detection;
pub mod locate_nozzle;
pub mod utilities;
pub mod vision_types;

use self::locate_nozzle::*;

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, bail, ensure, Context, Result};
use blob_detection::BlobDetectors;
use tracing::{debug, error, info, trace, warn};

use nokhwa::{
    pixel_format::RgbFormat,
    utils::{CameraFormat, RequestedFormat, RequestedFormatType},
};

use opencv::{
    core::Vector, features2d::SimpleBlobDetector, highgui, imgproc::ThresholdTypes, prelude::*,
    videoio,
};

pub use self::vision_types::*;
use crate::ui::data_labeling::SavedTargets;

pub fn spawn_locator_thread(
    ctx: egui::Context,
    mut handle: egui::TextureHandle,
    index: usize,
    mut channel_from_ui: crossbeam_channel::Receiver<WebcamCommand>,
    channel_to_ui: crossbeam_channel::Sender<WebcamMessage>,
    webcam_settings_mutex: Arc<Mutex<crate::vision::VisionSettings>>,
    camera_size: (f64, f64),
) {
    std::thread::spawn(move || {
        let format =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);

        let mut camera =
            nokhwa::Camera::new(nokhwa::utils::CameraIndex::Index(index as u32), format).unwrap();

        let format = CameraFormat::new(
            // nokhwa::utils::Resolution::new(Webcam::SIZE.0, Webcam::SIZE.1),
            nokhwa::utils::Resolution::new(camera_size.0 as u32, camera_size.1 as u32),
            nokhwa::utils::FrameFormat::MJPEG,
            30,
        );

        let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(format));

        camera.set_camera_requset(format).unwrap();

        // // Control: Brightness
        // // Control: Contrast
        // // Control: Hue
        // // Control: Saturation
        // // Control: Sharpness
        // // Control: Gamma
        // // Control: WhiteBalance
        // // Control: BacklightComp
        // // Control: Pan
        // // Control: Tilt
        // // Control: Zoom
        // // Control: Exposure
        // let controls = camera.supported_camera_controls().unwrap();
        // for control in controls {
        //     debug!("Control: {:?}", control);
        // }

        // for f in camera.compatible_camera_formats().unwrap() {
        //     debug!("Compatible format: {:?}", f);
        // }

        let mut buffer: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> =
            // image::ImageBuffer::new(Webcam::SIZE.0 as u32, Webcam::SIZE.1 as u32);
            image::ImageBuffer::new(camera_size.0 as u32, camera_size.1 as u32);

        let mut detectors = BlobDetectors::new().unwrap();

        let mut n = 0;

        let mut commands: VecDeque<WebcamCommand> = VecDeque::new();
        let mut screenshots = VecDeque::new();

        let mut saved_targets = {
            let path = "test_images/saved_targets.toml";
            if let Ok(s) = std::fs::read_to_string(&path) {
                let saved_targets: SavedTargets = toml::from_str(&s).unwrap();
                saved_targets
            } else {
                SavedTargets::default()
            }
        };

        eprintln!("Starting camera loop");
        loop {
            while let Ok(cmd) = channel_from_ui.try_recv() {
                match cmd {
                    WebcamCommand::SaveScreenshot(s) => {
                        screenshots.push_back(s);
                    }
                    WebcamCommand::SetCameraControl(cmd) => {
                        let c = cmd.to_control();

                        let control = camera.camera_control(c.0).unwrap();
                        // debug!("Control: {:?}", control);

                        camera.set_camera_control(c.0, c.1).unwrap();
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(34));
            // std::thread::sleep(std::time::Duration::from_millis(50));
            // std::thread::sleep(std::time::Duration::from_millis(200));
            // std::thread::sleep(std::time::Duration::from_millis(1000));

            let Ok(frame) = camera.frame() else {
                eprintln!("Failed to get frame");
                continue;
            };

            let res = frame.resolution();

            frame
                .decode_image_to_buffer::<RgbFormat>(&mut buffer)
                .unwrap();

            if let Some(cmd) = screenshots.pop_front() {
                match cmd {
                    None => {
                        let mut path = format!("test_images/frame_{:0>4}.jpg", n);
                        if std::path::Path::new(&path).exists() {
                            // increment n until we find a free name
                            while std::path::Path::new(&path).exists() {
                                n += 1;
                                path = format!("test_images/frame_{:0>4}.jpg", n);
                            }
                        }
                        n += 1;
                        debug!("Saving image to {}", path);

                        buffer.save(path).unwrap();
                    }
                    Some(pos) => {
                        let mut path = format!("test_images/frame_{:0>4}.jpg", saved_targets.index);
                        debug!("Saving image to {}", path);
                        let path = std::path::PathBuf::from(path);
                        assert!(!std::path::Path::new(&path).exists());
                        saved_targets.index += 1;
                        saved_targets.targets.insert(path.clone(), pos);

                        buffer.save(path).unwrap();

                        // save saved_targets to toml file
                        let path = "test_images/saved_targets.toml";
                        let s = toml::to_string(&saved_targets).unwrap();
                        std::fs::write(path, s).unwrap();
                    }
                }
            }

            let settings = webcam_settings_mutex.lock().unwrap().clone();

            // if settings.mirror.0 {
            //     image::imageops::flip_horizontal_in_place(&mut buffer);
            // }
            // if settings.mirror.1 {
            //     image::imageops::flip_vertical_in_place(&mut buffer);
            // }

            // match settings.rotate {
            //     0 => {}
            //     1 => {
            //         let buffer2 = buffer.clone();
            //         image::imageops::rotate90_in(&buffer2, &mut buffer).unwrap();
            //     }
            //     2 => image::imageops::rotate180_in_place(&mut buffer),
            //     3 => {
            //         let buffer2 = buffer.clone();
            //         image::imageops::rotate270_in(&buffer2, &mut buffer).unwrap();
            //     }
            //     _ => unimplemented!(),
            // }

            #[cfg(feature = "nope")]
            {
                /// Save images for debugging
                let mut img = utilities::imagebuffer_to_mat(&buffer).unwrap();

                let path = format!("test_images/frame_run0_{:0>4}", n);
                n += 1;
                // debug!("Saving image to {}", path);

                opencv::imgcodecs::imwrite(
                    &format!("test_images/frame_{:0>2}", n),
                    &img,
                    &opencv::core::Vector::new(),
                )
                .unwrap();
            }

            let t0 = std::time::Instant::now();

            match locate_nozzle(&mut buffer, &settings, &mut detectors) {
                Ok((img_out, circle)) => {
                    // debug!("Nozzle located");
                    utilities::mat_to_imagebuffer(&mut buffer, &img_out).unwrap();

                    if let Some(circle) = circle {
                        if channel_to_ui
                            .send(WebcamMessage::FoundNozzle(circle))
                            .is_err()
                        {
                            debug!("Failed to send message to UI");
                        }
                    } else {
                        if channel_to_ui.send(WebcamMessage::NozzleNotFound).is_err() {
                            debug!("Failed to send message to UI");
                        }
                    }
                }
                Err(e) => {
                    // eprintln!("Failed to locate nozzle: {}", e);
                    continue;
                }
            }

            #[cfg(feature = "nope")]
            if settings.draw_circle {
                if let Some(circle) = circle {
                    let mut img_color = Mat::new_rows_cols_with_default(
                        img.rows(),
                        img.cols(),
                        opencv::core::CV_8UC3,
                        0.0f64.into(),
                    )
                    .unwrap();

                    if img.data_bytes().unwrap().len() != buffer.len() {
                        opencv::imgproc::cvt_color(
                            &img,
                            &mut img_color,
                            // COLOR_BGR2GRAY,
                            opencv::imgproc::COLOR_GRAY2RGB,
                            0,
                            opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
                        )
                        .unwrap();
                        // std::mem::swap(&mut img, &mut img2);
                    } else {
                        img_color = img.clone();
                    }

                    // Draw the detected circle
                    let center = opencv::core::Point::new(circle.0 as i32, circle.1 as i32);
                    // let center = opencv::core::Point::new(322, 241);
                    let radius = circle.2 as i32;
                    // let radius = 20;

                    let color = opencv::core::Scalar::new(0., 255., 0., 0.);
                    let thickness = 2;

                    opencv::imgproc::circle(
                        &mut img_color,
                        center,
                        radius,
                        color,
                        thickness,
                        16,
                        0,
                    )
                    .unwrap();
                }
            }

            // let t1 = std::time::Instant::now();
            // let elapsed = t1.duration_since(t0);
            // debug!(
            //     "Elapsed time: {:.1} ms",
            //     elapsed.as_micros() as f64 / 1000.0
            // );

            let mut img = egui::ColorImage::from_rgb(
                [res.width() as usize, res.height() as usize],
                buffer.as_flat_samples().as_slice(),
            );

            crate::ui::webcam_controls::draw_crosshair(settings.crosshair_size, &mut img);

            handle.set(img, Default::default());

            ctx.request_repaint();

            //
        }
    });
}
