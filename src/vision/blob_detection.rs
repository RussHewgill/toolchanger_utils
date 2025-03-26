use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use opencv::{
    core::{Ptr, Vector},
    features2d::{SimpleBlobDetector, SimpleBlobDetector_Params},
};

#[derive(Debug)]
pub struct BlobDetectors {
    pub standard: Ptr<SimpleBlobDetector>,
    pub relaxed: Ptr<SimpleBlobDetector>,
    pub super_relaxed: Ptr<SimpleBlobDetector>,
    pub keypoints: Vector<opencv::core::KeyPoint>,
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
            min_area: std::f32::consts::PI * 20.0f32.powi(2), // ~1250
            // min_area: std::f32::consts::PI * 38.0f32.powi(2), // ~4500
            // max_area: std::f32::consts::PI * 100.0f32.powi(2), // 30_000
            // max_area: std::f32::consts::PI * 70.0f32.powi(2), // ~15400
            max_area: std::f32::consts::PI * 50.0f32.powi(2), // ~7800

            /// Filter by circularity
            filter_by_circularity: true,
            // filter_by_circularity: false,
            // min_circularity: 0.8,
            min_circularity: 0.4,
            max_circularity: 1.0,

            /// Filter by convexity
            filter_by_convexity: true,
            // filter_by_convexity: false,
            min_convexity: 0.3,
            // min_convexity: 0.5,
            max_convexity: 1.0,

            /// Filter by inertia
            filter_by_inertia: true,
            // filter_by_inertia: false,
            // min_inertia_ratio: 0.3, // kTAMV
            // min_inertia_ratio: 0.8,
            min_inertia_ratio: 0.5,
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

    fn blob_params_relaxed() -> SimpleBlobDetector_Params {
        SimpleBlobDetector_Params {
            /// Thresholds
            min_threshold: 1.0,
            max_threshold: 50.0,
            threshold_step: 1.0,

            /// Filter by area
            filter_by_area: true,
            // filter_by_area: false,
            min_area: std::f32::consts::PI * 20.0f32.powi(2), // ~1250
            max_area: std::f32::consts::PI * 125.0f32.powi(2), // 50_000

            /// Filter by circularity
            filter_by_circularity: true,
            min_circularity: 0.4,
            max_circularity: 1.0,

            /// Filter by convexity
            filter_by_convexity: true,
            min_convexity: 0.1,
            max_convexity: 1.0,

            /// Filter by inertia
            filter_by_inertia: true,
            min_inertia_ratio: 0.3, // kTAMV
            max_inertia_ratio: 340282350000000000000000000000000000000.,

            /// Filter by color
            filter_by_color: true,
            blob_color: 255,

            min_repeatability: 2,
            min_dist_between_blobs: 10.,
            collect_contours: false,
        }
    }

    fn blob_params_super_relaxed() -> SimpleBlobDetector_Params {
        SimpleBlobDetector_Params {
            /// Thresholds
            min_threshold: 20.0,
            max_threshold: 200.0,
            threshold_step: 1.0,

            /// Filter by area
            filter_by_area: true,
            // filter_by_area: false,
            min_area: 200.0,
            max_area: std::f32::consts::PI * 150.0f32.powi(2), // 70_000

            /// Filter by circularity
            filter_by_circularity: true,
            min_circularity: 0.5,
            max_circularity: 1.0,

            /// Filter by convexity
            filter_by_convexity: true,
            min_convexity: 0.5,
            max_convexity: 1.0,

            /// Filter by inertia
            filter_by_inertia: true,
            min_inertia_ratio: 0.5, // kTAMV
            max_inertia_ratio: 340282350000000000000000000000000000000.,

            /// Filter by color
            filter_by_color: true,
            blob_color: 255,

            min_repeatability: 2,
            min_dist_between_blobs: 2.,
            collect_contours: false,
        }
    }

    pub fn new() -> Result<Self> {
        let standard = SimpleBlobDetector::create(Self::blob_params_standard())?;
        let relaxed = SimpleBlobDetector::create(Self::blob_params_relaxed())?;
        let super_relaxed = SimpleBlobDetector::create(Self::blob_params_super_relaxed())?;

        // let p2 = SimpleBlobDetector_Params::default().unwrap();
        // debug!("max_inertia_ratio: {}", p2.max_inertia_ratio);

        Ok(Self {
            standard,
            relaxed,
            super_relaxed,
            keypoints: Vector::<opencv::core::KeyPoint>::new(),
        })
    }
}
