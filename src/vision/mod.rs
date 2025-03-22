use anyhow::{anyhow, bail, ensure, Context, Result};
use nokhwa::pixel_format::RgbFormat;
use tracing::{debug, error, info, trace, warn};

use nokhwa::utils::{CameraFormat, RequestedFormat, RequestedFormatType};
use opencv::imgproc::ThresholdTypes;
use opencv::prelude::*;
use opencv::{highgui, videoio};

use opencv::{
    core::{Size, Vec3f, Vector},
    imgcodecs::IMREAD_COLOR,
    imgproc::{
        cvt_color, gaussian_blur, hough_circles, threshold, COLOR_BGR2GRAY, HOUGH_GRADIENT,
        THRESH_BINARY_INV,
    },
    prelude::*,
};

use crate::webcam::Webcam;

#[cfg(feature = "nope")]
pub fn opencv_test() -> opencv::Result<()> {
    let window = "video capture";
    highgui::named_window(window, highgui::WINDOW_AUTOSIZE)?;
    let mut cam = videoio::VideoCapture::new(0, videoio::CAP_ANY)?; // 0 is the default camera
    let opened = videoio::VideoCapture::is_opened(&cam)?;

    if !opened {
        panic!("Unable to open default camera!");
    }

    loop {
        let mut frame = Mat::default();
        cam.read(&mut frame)?;

        // Write frame to file
        if frame.size()?.width > 0 {
            opencv::imgcodecs::imwrite("frame.jpg", &frame, &opencv::core::Vector::new())?;
            highgui::imshow(window, &frame)?;
        }

        if frame.size()?.width > 0 {
            highgui::imshow(window, &frame)?;
        }
        let key = highgui::wait_key(10)?;
        if key > 0 && key != 255 {
            break;
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum NozzlePosition {
    Centered,
    Up,
    Down,
    Left,
    Right,
    NotVisible,
}

pub fn spawn_locator_thread(ctx: egui::Context, mut handle: egui::TextureHandle, index: usize) {
    std::thread::spawn(move || {
        let format =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);

        let mut camera =
            nokhwa::Camera::new(nokhwa::utils::CameraIndex::Index(index as u32), format).unwrap();

        let format = CameraFormat::new(
            nokhwa::utils::Resolution::new(Webcam::SIZE.0, Webcam::SIZE.1),
            nokhwa::utils::FrameFormat::MJPEG,
            30,
        );

        let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(format));

        camera.set_camera_requset(format).unwrap();

        let mut buffer: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> =
            image::ImageBuffer::new(Webcam::SIZE.0 as u32, Webcam::SIZE.1 as u32);

        eprintln!("Starting camera loop");
        loop {
            std::thread::sleep(std::time::Duration::from_millis(50));
            // std::thread::sleep(std::time::Duration::from_millis(1000));

            let Ok(frame) = camera.frame() else {
                eprintln!("Failed to get frame");
                continue;
            };

            let res = frame.resolution();

            // debug!("Got frame: {}x{}", res.width(), res.height());

            frame
                .decode_image_to_buffer::<RgbFormat>(&mut buffer)
                .unwrap();

            if let Err(e) = locate_nozzle(&mut buffer) {
                eprintln!("Failed to locate nozzle: {}", e);
                continue;
            }

            let mut img = egui::ColorImage::from_rgb(
                [res.width() as usize, res.height() as usize],
                // &buffer[..res.width() as usize * res.height() as usize * 3],
                buffer.as_flat_samples().as_slice(),
            );

            handle.set(img, Default::default());

            ctx.request_repaint();

            //
        }
    });
}

pub fn locate_nozzle(img0: &mut image::ImageBuffer<image::Rgb<u8>, Vec<u8>>) -> Result<()> {
    // Convert image to opencv Mat

    let width = img0.width() as i32;
    let height = img0.height() as i32;
    let data = img0.as_raw().as_ptr();

    let mut img = unsafe {
        opencv::core::Mat::new_rows_cols_with_data_unsafe(
            height,
            width,
            opencv::core::CV_8UC3,
            data as *mut std::ffi::c_void,
            opencv::core::Mat_AUTO_STEP,
        )?
    };

    let mut img_out = img.clone();

    let mut img2 = img.clone();

    // debug!("img2 dimensions: {:?}", img2.size()?);
    // debug!("img2 channels: {:?}", img2.channels());

    cvt_color(
        &img,
        &mut img2,
        // COLOR_BGR2GRAY,
        opencv::imgproc::COLOR_RGB2YUV,
        0,
        opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
    )
    .unwrap();

    // extract Luma channel
    let mut yuv = Vector::<Mat>::new();
    opencv::core::split(&img2, &mut yuv)?;
    let y = yuv.get(0).unwrap();
    img2 = y.clone();

    // Apply Gaussian blur to reduce noise
    gaussian_blur(
        &img2,
        &mut img,
        Size::new(7, 7),
        1.5,
        1.5,
        opencv::core::BorderTypes::BORDER_REPLICATE.into(),
        opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
    )?;

    let thresh = 50.;
    // let thresh = 30.;

    // Threshold to isolate dark regions (nozzle)
    opencv::imgproc::adaptive_threshold(
        &img,
        &mut img2,
        255.,
        opencv::imgproc::ADAPTIVE_THRESH_GAUSSIAN_C.into(),
        ThresholdTypes::THRESH_BINARY.into(),
        35,
        1.,
    )?;

    // Detect circles using Hough Transform
    let mut circles: Vector<Vec3f> = Vector::new();
    hough_circles(
        &img2,
        &mut circles,
        HOUGH_GRADIENT,
        1.0,   // dp (inverse ratio of accumulator resolution)
        20.0,  // min_dist between circle centers
        100.0, // param1 (upper Canny edge threshold)
        30.0,  // param2 (accumulator threshold)
        20,    // min_radius
        50,    // max_radius
    )?;

    // debug!("Circles detected: {}", circles.len());

    // Filter and select the best candidate
    let (img_w, img_h) = (img.cols() as f32, img.rows() as f32);
    let (center_x, center_y) = (img_w / 2.0, img_h / 2.0);
    let mut best_circle: Option<opencv::core::Vec3f> = None;
    const RADIUS_RANGE: (f32, f32) = (10.0, 50.0);

    for circle in circles.iter() {
        let radius = circle[2];
        if radius < RADIUS_RANGE.0 || radius > RADIUS_RANGE.1 {
            continue;
        }

        // Prefer circles closer to the center if similar size
        let current_center_dist =
            ((circle[0] - center_x).powi(2) + (circle[1] - center_y).powi(2)).sqrt();
        if let Some(best) = best_circle {
            let best_center_dist =
                ((best[0] - center_x).powi(2) + (best[1] - center_y).powi(2)).sqrt();
            if radius > best[2] || (radius == best[2] && current_center_dist < best_center_dist) {
                best_circle = Some(circle);
            }
        } else {
            best_circle = Some(circle);
        }
    }

    if let Some(circle) = best_circle {
        // let gray = img.clone();
        // cvt_color(
        //     &gray,
        //     &mut img2,
        //     // COLOR_BGR2GRAY,
        //     opencv::imgproc::COLOR_GRAY2RGB,
        //     0,
        //     opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
        // )
        // .unwrap();

        // Draw the detected circle
        let center = opencv::core::Point::new(circle[0] as i32, circle[1] as i32);
        // let center = opencv::core::Point::new(322, 241);
        // let radius = circle[2] as i32;
        let radius = 20;

        let color = opencv::core::Scalar::new(0., 255., 0., 0.);
        let thickness = 2;

        opencv::imgproc::circle(&mut img_out, center, radius, color, thickness, 16, 0)?;
    }

    // cvt_color(
    //     &img2,
    //     &mut img,
    //     // COLOR_BGR2GRAY,
    //     // opencv::imgproc::COLOR_RGB2YUV,
    //     opencv::imgproc::COLOR_YUV2RGB,
    //     0,
    //     opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
    // )
    // .unwrap();

    // copy image back to ImageBuffer
    img0.copy_from_slice(img_out.data_bytes()?);

    Ok(())
}

pub fn _locate_nozzle(image_path: &str) -> opencv::Result<NozzlePosition> {
    let mut img = opencv::imgcodecs::imread(image_path, IMREAD_COLOR)?;
    if img.empty() {
        return Ok(NozzlePosition::NotVisible);
    }

    let mut img2 = img.clone();

    // Convert to grayscale
    cvt_color(
        &img,
        &mut img2,
        // COLOR_BGR2GRAY,
        opencv::imgproc::COLOR_BGR2YUV,
        0,
        opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
    )
    .unwrap();

    // debug!("img2 dimensions: {:?}", img2.size()?);
    // debug!("img2 channels: {:?}", img2.channels());

    // let y = opencv::gapi::split3(&img2)?.get_0();

    let mut yuv = Vector::<Mat>::new();

    opencv::core::split(&img2, &mut yuv)?;

    let y = yuv.get(0).unwrap();

    img2 = y.clone();

    debug!("y dimensions: {:?}", y.size()?);
    debug!("y channels: {:?}", y.channels());

    // opencv::core::mix_channels(&img2, &mut img, &[]);
    // std::mem::swap(&mut img, &mut img2);

    // Apply Gaussian blur to reduce noise
    gaussian_blur(
        &img2,
        &mut img,
        Size::new(7, 7),
        1.5,
        1.5,
        opencv::core::BorderTypes::BORDER_REPLICATE.into(),
        opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
    )?;

    let thresh = 50.;
    // let thresh = 30.;

    // // Threshold to isolate dark regions (nozzle)
    // threshold(&img, &mut img2, thresh, 255.0, THRESH_BINARY_INV)?;

    opencv::imgproc::adaptive_threshold(
        &img,
        &mut img2,
        255.,
        opencv::imgproc::ADAPTIVE_THRESH_GAUSSIAN_C.into(),
        ThresholdTypes::THRESH_BINARY.into(),
        35,
        // 9,
        1.,
    )?;

    // Detect circles using Hough Transform
    let mut circles: Vector<Vec3f> = Vector::new();
    hough_circles(
        &img2,
        &mut circles,
        HOUGH_GRADIENT,
        1.0,   // dp (inverse ratio of accumulator resolution)
        20.0,  // min_dist between circle centers
        100.0, // param1 (upper Canny edge threshold)
        30.0,  // param2 (accumulator threshold)
        20,    // min_radius
        50,    // max_radius
    )?;

    debug!("Circles detected: {}", circles.len());
    for c in circles.iter() {
        debug!("Circle: {:?}", c);
    }

    // Filter and select the best candidate
    let (img_w, img_h) = (img.cols() as f32, img.rows() as f32);
    let (center_x, center_y) = (img_w / 2.0, img_h / 2.0);
    let mut best_circle: Option<opencv::core::Vec3f> = None;
    const RADIUS_RANGE: (f32, f32) = (10.0, 50.0);

    for circle in circles.iter() {
        let radius = circle[2];
        if radius < RADIUS_RANGE.0 || radius > RADIUS_RANGE.1 {
            continue;
        }

        // Prefer circles closer to the center if similar size
        let current_center_dist =
            ((circle[0] - center_x).powi(2) + (circle[1] - center_y).powi(2)).sqrt();
        if let Some(best) = best_circle {
            let best_center_dist =
                ((best[0] - center_x).powi(2) + (best[1] - center_y).powi(2)).sqrt();
            if radius > best[2] || (radius == best[2] && current_center_dist < best_center_dist) {
                best_circle = Some(circle);
            }
        } else {
            best_circle = Some(circle);
        }
    }

    // Write frame to file
    // opencv::imgcodecs::imwrite("output.jpg", &img, &opencv::core::Vector::new())?;
    opencv::imgcodecs::imwrite("output.jpg", &img2, &opencv::core::Vector::new())?;

    let Some(circle) = best_circle else {
        // let window = "video capture";
        // highgui::named_window(window, highgui::WINDOW_AUTOSIZE)?;

        // if img.size()?.width > 0 {
        //     highgui::imshow(window, &img2)?;
        // }
        // let key = highgui::wait_key(10_000)?;

        return Ok(NozzlePosition::NotVisible);
    };

    debug!("Best circle: {:?}", circle);

    // #[cfg(feature = "nope")]
    {
        let gray = img.clone();
        // let mut gray = Mat::default();
        cvt_color(
            &gray,
            &mut img2,
            // COLOR_BGR2GRAY,
            opencv::imgproc::COLOR_GRAY2BGR,
            0,
            opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
        )
        .unwrap();

        // Draw the detected circle
        let center = opencv::core::Point::new(circle[0] as i32, circle[1] as i32);
        // let center = opencv::core::Point::new(322, 241);
        // let radius = circle[2] as i32;
        let radius = 20;

        let color = opencv::core::Scalar::new(0., 255., 0., 0.); // Green

        // let color = opencv::core::Scalar::new(255., 255., 255., 0.); // Green
        let thickness = 2;

        opencv::imgproc::circle(&mut img2, center, radius, color, thickness, 16, 0)?;

        // Write frame to file
        // opencv::imgcodecs::imwrite("output.jpg", &img, &opencv::core::Vector::new())?;
        opencv::imgcodecs::imwrite("output.jpg", &img2, &opencv::core::Vector::new())?;
    }

    let window = "video capture";
    highgui::named_window(window, highgui::WINDOW_AUTOSIZE)?;

    if img.size()?.width > 0 {
        highgui::imshow(window, &img2)?;
    }
    let key = highgui::wait_key(10_000)?;
    // if key > 0 && key != 255 {
    //     break;
    // }

    Ok(NozzlePosition::NotVisible)
}
