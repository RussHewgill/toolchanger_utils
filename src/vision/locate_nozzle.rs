use anyhow::{anyhow, bail, ensure, Context, Result};
use opencv::core::Point;
use opencv::imgproc;
use tracing::{debug, error, info, trace, warn};

use super::blob_detection::BlobDetectors;
use super::utilities;
use super::WebcamSettings;

use opencv::{
    core::{Ptr, Size, Vec3f, Vector},
    features2d::{SimpleBlobDetector, SimpleBlobDetector_Params},
    imgproc::{cvt_color, gaussian_blur, hough_circles, threshold, ThresholdTypes},
    prelude::*,
};

/// Algorithms from kTAMV:
/// Algo 0:
/// 1. Convert image to YUV color space, extract Y channel
/// 2. Apply Gaussian blur to reduce noise (7x7 kernel, 6 sigma)
/// 3. Adaptive threshold to isolate dark regions (thresh = 35, constant = 1)
///
/// Algo 1:
/// 1. Convert image to grayscale
/// 2. Threshold using triangle method (127, 255)
/// 3. Apply Gaussian blur (7x7 kernel, 6 sigma)
///
/// Algo 2:
/// 1. Convert image to grayscale
/// 2. Apply median blur (5x5 kernel)

#[cfg(feature = "nope")]
pub fn locate_nozzle(
    img0: &mut image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    settings: &WebcamSettings,
    detectors: &mut BlobDetectors,
) -> Result<(Mat, Option<(f64, f64, f64)>)> {
    // Convert image to opencv Mat

    let mut img = utilities::imagebuffer_to_mat(img0)?;

    let mut img_out = img.clone();

    let mut img2 = img.clone();

    // Adjust gamma to 1.2
    let gamma = 1.2;
    let mut lut = Mat::new_rows_cols_with_default(1, 256, opencv::core::CV_8U, 0.0f64.into())?;
    for i in 0..256 {
        let value = ((i as f64 / 255.0).powf(1.0 / gamma) * 255.0) as u8;
        *lut.at_mut::<u8>(i)? = value;
    }

    opencv::core::lut(&img, &lut, &mut img2)?;
    std::mem::swap(&mut img, &mut img2);

    if settings.filter_step == 0 {
        // debug!("Filter step is 0, skipping filter");
        img_out = img.clone();
    }

    cvt_color(
        &img,
        &mut img2,
        opencv::imgproc::COLOR_RGB2YUV,
        0,
        opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
    )
    .unwrap();
    std::mem::swap(&mut img, &mut img2);

    // extract Luma channel
    let mut yuv = Vector::<Mat>::new();
    opencv::core::split(&img, &mut yuv)?;
    let y = yuv.get(0).unwrap();
    img = y.clone();

    if settings.filter_step == 1 {
        // debug!("Filter step is 1, returning luma channel");
        img_out = img.clone();
    }

    // Apply Gaussian blur to reduce noise
    gaussian_blur(
        &img,
        &mut img2,
        Size::new(settings.blur_kernel_size, settings.blur_kernel_size),
        settings.blur_sigma,
        settings.blur_sigma,
        opencv::core::BorderTypes::BORDER_REPLICATE.into(),
        opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
    )?;
    std::mem::swap(&mut img, &mut img2);

    if settings.filter_step == 2 {
        // debug!("Filter step is 2, returning blurred image");
        img_out = img.clone();
    }

    // Threshold to isolate dark regions (nozzle)
    if settings.adaptive_threshold {
        opencv::imgproc::adaptive_threshold(
            &img,
            &mut img2,
            255.,
            opencv::imgproc::ADAPTIVE_THRESH_GAUSSIAN_C.into(),
            ThresholdTypes::THRESH_BINARY.into(),
            // thresh,
            settings.threshold_block_size * 2 + 1,
            // settings.adaptive_threshold_c as f64,
            1.,
        )?;
    } else {
        // threshold(
        //     &img,
        //     &mut img2,
        //     settings.adaptive_threshold_block_size as f64 * 2. + 1.,
        //     255.0,
        //     opencv::imgproc::THRESH_BINARY_INV,
        // )?;

        let t = match settings.threshold_type {
            0 => opencv::imgproc::THRESH_BINARY_INV,
            1 => opencv::imgproc::THRESH_BINARY_INV + opencv::imgproc::THRESH_TRIANGLE,
            2 => opencv::imgproc::THRESH_BINARY_INV + opencv::imgproc::THRESH_OTSU,
            _ => bail!("Invalid threshold type"),
        };

        threshold(
            &img,
            &mut img2,
            // settings.threshold_block_size as f64 * 2. + 1.,
            settings.threshold_block_size as f64,
            255.0,
            t,
        )?;
    }
    std::mem::swap(&mut img, &mut img2);

    #[cfg(feature = "nope")]
    if false {
        let kernel = imgproc::get_structuring_element(
            imgproc::MORPH_ELLIPSE,
            Size::new(5, 5),
            Point::new(-1, -1),
        )?;

        imgproc::morphology_ex(
            &img,
            &mut img2,
            imgproc::MORPH_CLOSE,
            &kernel,
            Point::new(-1, -1),
            2,
            opencv::core::BORDER_CONSTANT,
            imgproc::morphology_default_border_value()?,
        )?;
        std::mem::swap(&mut img, &mut img2);
    }

    if settings.filter_step == 3 {
        // debug!("Filter step is 3, returning thresholded image");
        img_out = img.clone();
    }

    let mut best_circle: Option<opencv::core::Vec3f> = None;

    if settings.use_hough {
        // debug!("Using Hough Transform for circle detection");

        // Detect circles using Hough Transform
        let mut circles: Vector<Vec3f> = Vector::new();
        hough_circles(
            &img,
            &mut circles,
            imgproc::HOUGH_GRADIENT,
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
                if radius > best[2] || (radius == best[2] && current_center_dist < best_center_dist)
                {
                    best_circle = Some(circle);
                }
            } else {
                best_circle = Some(circle);
            }
        }
    } else {
        detectors.keypoints.clear();
        detectors
            .standard
            .detect(&img, &mut detectors.keypoints, &opencv::core::no_array())?;

        // if detectors.keypoints.len() == 0 {
        //     // debug!("Standard detector found no keypoints, trying relaxed detector");
        //     detectors
        //         .relaxed
        //         .detect(&img, &mut detectors.keypoints, &opencv::core::no_array())?;
        // } else {
        //     debug!(
        //         "Standard detector found {} keypoints",
        //         detectors.keypoints.len()
        //     );
        // }

        // if detectors.keypoints.len() == 0 {
        //     // debug!("Relaxed detector found no keypoints, trying super relaxed detector");
        //     detectors.super_relaxed.detect(
        //         &img,
        //         &mut detectors.keypoints,
        //         &opencv::core::no_array(),
        //     )?;
        // } else {
        //     debug!(
        //         "Relaxed detector found {} keypoints",
        //         detectors.keypoints.len()
        //     );
        // }

        // debug!("Keypoints detected: {}", detectors.keypoints.len());

        if let Ok(keypoint) = detectors.keypoints.get(0) {
            let x = keypoint.pt().x;
            let y = keypoint.pt().y;
            let radius = keypoint.size() / 2.0;

            // debug!("Keypoint: ({}, {}), radius: {}", x, y, radius);

            best_circle = Some(opencv::core::Vec3f::from_array([
                x as f32, y as f32, radius,
            ]));

            // let area = keypoint.size().powi(2) * std::f32::consts::PI;
            // debug!("area = {:.0}", area);
        }

        //
    }

    if settings.filter_step > 3 {
        // debug!("Filter step is {}, returning circles", settings.filter_step);
        img_out = img.clone();
    }

    if let Some(circle) = best_circle {
        let mut img_color = Mat::new_rows_cols_with_default(
            img.rows(),
            img.cols(),
            opencv::core::CV_8UC3,
            0.0f64.into(),
        )?;

        if img_out.data_bytes().unwrap().len() != img0.len() {
            cvt_color(
                &img_out,
                &mut img_color,
                // COLOR_BGR2GRAY,
                opencv::imgproc::COLOR_GRAY2RGB,
                0,
                opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
            )
            .unwrap();
            // std::mem::swap(&mut img, &mut img2);
        } else {
            img_color = img_out.clone();
        }

        // Draw the detected circle
        let center = opencv::core::Point::new(circle[0] as i32, circle[1] as i32);
        // let center = opencv::core::Point::new(322, 241);
        let radius = circle[2] as i32;
        // let radius = 20;

        let color = opencv::core::Scalar::new(0., 255., 0., 0.);
        let thickness = 2;

        if settings.draw_circle {
            opencv::imgproc::circle(&mut img_color, center, radius, color, thickness, 16, 0)?;
        }

        let circle = (circle[0] as f64, circle[1] as f64, circle[2] as f64);
        return Ok((img_color, Some(circle)));
    }

    Ok((img_out, None))
}

// #[cfg(feature = "nope")]
pub fn locate_nozzle(
    img0: &mut image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    settings: &WebcamSettings,
    detectors: &mut BlobDetectors,
) -> Result<(Mat, Option<(f64, f64, f64)>)> {
    let mut img = utilities::imagebuffer_to_mat(img0)?;
    let mut img2 = img.clone();

    // Adjust gamma to 1.2
    let gamma = 1.2;
    let mut lut = Mat::new_rows_cols_with_default(1, 256, opencv::core::CV_8U, 0.0f64.into())?;
    for i in 0..256 {
        let value = ((i as f64 / 255.0).powf(1.0 / gamma) * 255.0) as u8;
        *lut.at_mut::<u8>(i)? = value;
    }

    opencv::core::lut(&img, &lut, &mut img2)?;
    std::mem::swap(&mut img, &mut img2);

    let (mut img_out, mat0) = preprocess_0(&img, settings, 0)?;
    let (_, mat1) = preprocess_0(&img, settings, 1)?;
    // let mat1 = preprocess_1(&img, settings)?;
    // let mat2 = preprocess_2(&img, settings)?;
    drop(img);
    drop(img2);

    let mut best_circle: Option<(f64, f64, f64)> = None;

    // let mut mat = match settings.preprocess_pipeline {
    //     0 => mat0.clone(),
    //     1 => mat1.clone(),
    //     // 2 => mat2,
    //     _ => bail!("Invalid preprocess pipeline"),
    // };

    /// Find keypoints
    if settings.use_hough {
        detectors.keypoints.clear();

        detectors
            .standard
            .detect(&mat0, &mut detectors.keypoints, &opencv::core::no_array())?;

        if detectors.keypoints.len() == 0 {
            detectors.standard.detect(
                &mat1,
                &mut detectors.keypoints,
                &opencv::core::no_array(),
            )?;
        } else {
            // debug!("mat0: found {}", detectors.keypoints.len());
        }

        if detectors.keypoints.len() > 0 {
            // debug!("mat1: found {}", detectors.keypoints.len());
        }

        // if detectors.keypoints.len() == 0 {
        //     detectors
        //         .relaxed
        //         .detect(&mat, &mut detectors.keypoints, &opencv::core::no_array())?;
        //     if detectors.keypoints.len() > 0 {
        //         // debug!(
        //         //     "Relaxed detector found {} keypoints",
        //         //     detectors.keypoints.len()
        //         // );
        //     }
        // } else {
        //     // debug!(
        //     //     "Standard detector found {} keypoints",
        //     //     detectors.keypoints.len()
        //     // );
        // }

        // if detectors.keypoints.len() == 0 {
        //     detectors.super_relaxed.detect(
        //         &img,
        //         &mut detectors.keypoints,
        //         &opencv::core::no_array(),
        //     )?;
        //     if detectors.keypoints.len() > 0 {
        //         debug!(
        //             "Super relaxed detector found {} keypoints",
        //             detectors.keypoints.len()
        //         );
        //     }
        // }

        if let Ok(keypoint) = detectors.keypoints.get(0) {
            let x = keypoint.pt().x;
            let y = keypoint.pt().y;
            let radius = keypoint.size() / 2.0;

            // debug!("Keypoint: ({}, {}), radius: {}", x, y, radius);

            best_circle = Some((x as f64, y as f64, radius as f64));

            if settings.draw_circle {
                let mut img_color = Mat::new_rows_cols_with_default(
                    img_out.rows(),
                    img_out.cols(),
                    opencv::core::CV_8UC3,
                    0.0f64.into(),
                )?;

                if img_out.data_bytes().unwrap().len() != img0.len() {
                    cvt_color(
                        &img_out,
                        &mut img_color,
                        // COLOR_BGR2GRAY,
                        opencv::imgproc::COLOR_GRAY2RGB,
                        0,
                        opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
                    )
                    .unwrap();
                    // std::mem::swap(&mut img, &mut img2);
                } else {
                    img_color = img_out.clone();
                }

                let center = opencv::core::Point::new(x as i32, y as i32);
                let radius = radius as i32;
                // let center = opencv::core::Point::new(322, 241);
                let color = opencv::core::Scalar::new(0., 255., 0., 0.);
                let thickness = 2;
                opencv::imgproc::circle(&mut img_color, center, radius, color, thickness, 16, 0)?;

                img_out = img_color;
            }

            // let area = keypoint.size().powi(2) * std::f32::consts::PI;
            // debug!("area = {:.0}", area);
        }
    }

    Ok((img_out, best_circle))
}

pub fn preprocess_0(
    img: &Mat,
    settings: &WebcamSettings,
    thresh_type: usize,
) -> Result<(Mat, Mat)> {
    let mut img = img.clone();
    let mut img2 = img.clone();
    let mut img_out = img.clone();

    if settings.filter_step == 0 {
        // debug!("Filter step is 0, skipping filter");
        img_out = img.clone();
    }

    cvt_color(
        &img,
        &mut img2,
        opencv::imgproc::COLOR_RGB2YUV,
        0,
        opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
    )
    .unwrap();
    std::mem::swap(&mut img, &mut img2);

    // extract Luma channel
    let mut yuv = Vector::<Mat>::new();
    opencv::core::split(&img, &mut yuv)?;
    let y = yuv.get(0).unwrap();
    img = y.clone();

    if settings.filter_step == 1 {
        // debug!("Filter step is 1, returning luma channel");
        img_out = img.clone();
    }

    // Apply Gaussian blur to reduce noise
    gaussian_blur(
        &img,
        &mut img2,
        Size::new(settings.blur_kernel_size, settings.blur_kernel_size),
        settings.blur_sigma,
        settings.blur_sigma,
        opencv::core::BorderTypes::BORDER_REPLICATE.into(),
        opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
    )?;
    std::mem::swap(&mut img, &mut img2);

    if settings.filter_step == 2 {
        // debug!("Filter step is 2, returning blurred image");
        img_out = img.clone();
    }

    // Threshold to isolate dark regions (nozzle)
    if settings.adaptive_threshold {
        opencv::imgproc::adaptive_threshold(
            &img,
            &mut img2,
            255.,
            opencv::imgproc::ADAPTIVE_THRESH_GAUSSIAN_C.into(),
            ThresholdTypes::THRESH_BINARY.into(),
            // thresh,
            settings.threshold_block_size * 2 + 1,
            // settings.adaptive_threshold_c as f64,
            1.,
        )?;
    } else {
        // threshold(
        //     &img,
        //     &mut img2,
        //     settings.adaptive_threshold_block_size as f64 * 2. + 1.,
        //     255.0,
        //     opencv::imgproc::THRESH_BINARY_INV,
        // )?;

        let t = match thresh_type {
            0 => opencv::imgproc::THRESH_BINARY_INV,
            1 => opencv::imgproc::THRESH_BINARY_INV + opencv::imgproc::THRESH_TRIANGLE,
            2 => opencv::imgproc::THRESH_BINARY_INV + opencv::imgproc::THRESH_OTSU,
            _ => bail!("Invalid threshold type"),
        };

        threshold(
            &img,
            &mut img2,
            // settings.threshold_block_size as f64 * 2. + 1.,
            settings.threshold_block_size as f64,
            255.0,
            t,
        )?;
    }
    std::mem::swap(&mut img, &mut img2);

    if settings.filter_step == 3 {
        // debug!("Filter step is 3, returning thresholded image");
        img_out = img.clone();
    }

    Ok((img_out, img))
}

/// 1. Convert image to YUV color space, extract Y channel
/// 2. Gaussian blur
/// 3. Adaptive threshold
pub fn preprocess_1(img: &Mat, settings: &WebcamSettings) -> Result<Mat> {
    let mut img = img.clone();
    let mut img2 = img.clone();

    // extract Luma channel
    cvt_color(
        &img,
        &mut img2,
        opencv::imgproc::COLOR_RGB2YUV,
        0,
        opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
    )
    .unwrap();
    std::mem::swap(&mut img, &mut img2);

    let mut yuv = Vector::<Mat>::new();
    opencv::core::split(&img, &mut yuv)?;
    let y = yuv.get(0).unwrap();
    img = y.clone();

    // Apply Gaussian blur to reduce noise
    gaussian_blur(
        &img,
        &mut img2,
        Size::new(7, 7),
        6.,
        6.,
        opencv::core::BorderTypes::BORDER_REPLICATE.into(),
        opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
    )?;
    std::mem::swap(&mut img, &mut img2);

    opencv::imgproc::adaptive_threshold(
        &img,
        &mut img2,
        255.,
        opencv::imgproc::ADAPTIVE_THRESH_GAUSSIAN_C.into(),
        ThresholdTypes::THRESH_BINARY.into(),
        35,
        1.,
    )?;
    std::mem::swap(&mut img, &mut img2);

    Ok(img)
}

/// 1. Convert image to grayscale
/// 2. Threshold using triangle method (127, 255)
/// 3. Apply Gaussian blur (7x7 kernel, 6 sigma)
pub fn preprocess_2(img: &Mat, settings: &WebcamSettings) -> Result<Mat> {
    let mut img = img.clone();
    let mut img2 = img.clone();

    cvt_color(
        &img,
        &mut img2,
        opencv::imgproc::COLOR_RGB2GRAY,
        0,
        opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
    )
    .unwrap();
    std::mem::swap(&mut img, &mut img2);

    threshold(
        &img,
        &mut img2,
        127.0,
        255.0,
        // opencv::imgproc::THRESH_BINARY_INV + opencv::imgproc::THRESH_TRIANGLE,
        opencv::imgproc::THRESH_BINARY + opencv::imgproc::THRESH_TRIANGLE,
    )?;
    std::mem::swap(&mut img, &mut img2);

    // Apply Gaussian blur to reduce noise
    gaussian_blur(
        &img,
        &mut img2,
        Size::new(7, 7),
        6.,
        6.,
        opencv::core::BorderTypes::BORDER_REPLICATE.into(),
        opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
    )?;
    std::mem::swap(&mut img, &mut img2);

    Ok(img)
}

/// CLAHE, doesn't work well
#[cfg(feature = "nope")]
pub fn preprocess_2(img: &Mat, settings: &WebcamSettings) -> Result<Mat> {
    let mut img = img.clone();
    let mut img2 = img.clone();
    let mut img_out = img.clone();

    let mut clahe = imgproc::create_clahe(40.0, Size::new(8, 8))?;
    clahe.apply(&img, &mut img2)?;
    std::mem::swap(&mut img, &mut img2);

    imgproc::median_blur(&img, &mut img2, 7)?;
    std::mem::swap(&mut img, &mut img2);

    opencv::imgproc::adaptive_threshold(
        &img,
        &mut img2,
        255.,
        opencv::imgproc::ADAPTIVE_THRESH_GAUSSIAN_C.into(),
        ThresholdTypes::THRESH_BINARY.into(),
        11,
        2.,
    )?;
    std::mem::swap(&mut img, &mut img2);

    Ok(img)
}
