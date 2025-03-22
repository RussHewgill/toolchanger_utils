use anyhow::{anyhow, bail, ensure, Context, Result};
use opencv::core::Ptr;
use opencv::core::Size;
use opencv::core::Vec3f;
use opencv::core::Vector;
use opencv::features2d::SimpleBlobDetector;
use opencv::features2d::SimpleBlobDetector_Params;
use opencv::imgproc::cvt_color;
use opencv::imgproc::gaussian_blur;
use opencv::imgproc::hough_circles;
use opencv::imgproc::threshold;
use opencv::imgproc::ThresholdTypes;
use tracing::{debug, error, info, trace, warn};

use super::utilities;
use super::WebcamSettings;

use opencv::prelude::*;

#[derive(Debug)]
pub struct BlobDetectors {
    pub standard: Ptr<SimpleBlobDetector>,
    // pub relaxed: SimpleBlobDetector,
    // pub super_relaxed: SimpleBlobDetector,
    keypoints: Vector<opencv::core::KeyPoint>,
}

impl BlobDetectors {
    fn blob_params_standard() -> SimpleBlobDetector_Params {
        SimpleBlobDetector_Params {
        /// Thresholds
        min_threshold: 1.0,
        max_threshold: 50.0,
        threshold_step: 1.0,

        /// Filter by area
        filter_by_area: true,
        // filter_by_area: false,
        // min_area: 400.0,
        // max_area: 900.0,
        // max_area: 15000.0,
        min_area: std::f32::consts::PI * 20.0f32.powi(2), // ~5000
        max_area: std::f32::consts::PI * 100.0f32.powi(2), // ~6300

        /// Filter by circularity
        filter_by_circularity: true,
        // filter_by_circularity: false,
        // min_circularity: 0.8,
        min_circularity: 0.4,
        max_circularity: 1.0,

        /// Filter by convexity
        // filter_by_convexity: true,
        filter_by_convexity: false,
        min_convexity: 0.3,
        max_convexity: 1.0,

        /// Filter by inertia
        filter_by_inertia: true,
        // filter_by_inertia: false,
        // min_inertia_ratio: 0.3, // kTAMV
        // min_inertia_ratio: 0.8,
        min_inertia_ratio: 0.1,
        // max_inertia_ratio: f32::INFINITY,
        max_inertia_ratio: 340282350000000000000000000000000000000.,

        /// Filter by color
        filter_by_color: true,
        // filter_by_color: false,
        blob_color: 255,

        min_repeatability: 2,
        min_dist_between_blobs: 10.,
        collect_contours: false,
    }
    }

    pub fn new() -> Result<Self> {
        let params = Self::blob_params_standard();
        let standard = SimpleBlobDetector::create(params)?;

        // let p2 = SimpleBlobDetector_Params::default().unwrap();
        // debug!("max_inertia_ratio: {}", p2.max_inertia_ratio);

        Ok(Self {
            standard,
            // relaxed,
            // super_relaxed,
            keypoints: Vector::<opencv::core::KeyPoint>::new(),
        })
    }
}

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

pub fn locate_nozzle(
    img0: &mut image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    settings: &WebcamSettings,
    detectors: &mut BlobDetectors,
) -> Result<(Mat, Option<(f64, f64, f64)>)> {
    // Convert image to opencv Mat

    let mut img = utilities::imagebuffer_to_mat(img0)?;

    let mut img_out = img.clone();

    let mut img2 = img.clone();

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

    if settings.adaptive_threshold {
        // Threshold to isolate dark regions (nozzle)
        opencv::imgproc::adaptive_threshold(
            &img,
            &mut img2,
            255.,
            opencv::imgproc::ADAPTIVE_THRESH_GAUSSIAN_C.into(),
            ThresholdTypes::THRESH_BINARY.into(),
            // thresh,
            settings.adaptive_threshold_block_size,
            settings.adaptive_threshold_c as f64,
        )?;
    } else {
        threshold(
            &img,
            &mut img2,
            settings.adaptive_threshold_block_size as f64 * 2. + 1.,
            255.0,
            opencv::imgproc::THRESH_BINARY_INV,
        )?;
    }
    std::mem::swap(&mut img, &mut img2);

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
            opencv::imgproc::HOUGH_GRADIENT,
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

        debug!("Keypoints detected: {}", detectors.keypoints.len());

        if let Ok(keypoint) = detectors.keypoints.get(0) {
            let x = keypoint.pt().x;
            let y = keypoint.pt().y;
            let radius = keypoint.size() / 2.0;

            debug!("Keypoint: ({}, {}), radius: {}", x, y, radius);

            best_circle = Some(opencv::core::Vec3f::from_array([
                x as f32, y as f32, radius,
            ]));
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
