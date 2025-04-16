use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use opencv::{
    core::{Ptr, Vector},
    features2d::{SimpleBlobDetector, SimpleBlobDetector_Params},
};

#[derive(Debug)]
pub struct BlobDetectors {
    pub params_standard: SimpleBlobDetector_Params,
    pub params_relaxed: SimpleBlobDetector_Params,
    pub params_super_relaxed: SimpleBlobDetector_Params,

    pub standard: Ptr<SimpleBlobDetector>,
    pub relaxed: Ptr<SimpleBlobDetector>,
    pub super_relaxed: Ptr<SimpleBlobDetector>,
    pub keypoints: Vector<opencv::core::KeyPoint>,
}

impl BlobDetectors {
    /// Optimized, pass 2
    /// [51.994232, 53.994232, 1000.0, 50000.0, 0.758737, 0.8, 0.2]
    fn blob_params_standard() -> SimpleBlobDetector_Params {
        SimpleBlobDetector_Params {
            /// Thresholds
            min_threshold: 50.0,
            max_threshold: 100.0,
            // max_threshold: 50.0,
            // threshold_step: 1.0,
            threshold_step: 25.0,

            filter_by_area: true,
            // min_area: 1000.,
            // min_area: 2000.,
            min_area: 3000.,
            max_area: 50_000.,

            filter_by_circularity: true,
            min_circularity: 0.75,
            max_circularity: 1.0,

            filter_by_convexity: true,
            min_convexity: 0.8,
            max_convexity: 1.0,

            filter_by_inertia: true,
            min_inertia_ratio: 0.2,
            max_inertia_ratio: 340282350000000000000000000000000000000.,

            /// Filter by color
            filter_by_color: true,
            blob_color: 255, // white
            // blob_color: 0, // black

            // min_repeatability: 2,
            min_repeatability: 1,
            min_dist_between_blobs: 10.,
            collect_contours: false,
        }
    }

    #[cfg(feature = "nope")]
    /// Optimized, pass 1
    fn blob_params_standard() -> SimpleBlobDetector_Params {
        SimpleBlobDetector_Params {
            /// Thresholds
            min_threshold: 1.0,
            max_threshold: 250.0,
            threshold_step: 1.0,

            /// Filter by area
            filter_by_area: true,
            // filter_by_area: false,
            // min_area: 400.0,
            // max_area: 900.0,
            // max_area: 15000.0,
            min_area: 2205.,
            max_area: std::f32::consts::PI * 60.0f32.powi(2),

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
            blob_color: 255, // white
            // blob_color: 0, // black
            min_repeatability: 2,
            min_dist_between_blobs: 10.,
            collect_contours: false,
        }
    }

    /// Hand-tuned
    #[cfg(feature = "nope")]
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
            // max_area: std::f32::consts::PI * 70.0f32.powi(2), // ~15400
            // max_area: std::f32::consts::PI * 50.0f32.powi(2), // ~7800
            max_area: std::f32::consts::PI * 100.0f32.powi(2), // 30_000

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

    /// XXX: uses same params for all solvers
    pub fn new_with_params(params: SimpleBlobDetector_Params) -> Result<Self> {
        let standard = SimpleBlobDetector::create(params.clone())?;
        let relaxed = SimpleBlobDetector::create(params.clone())?;
        let super_relaxed = SimpleBlobDetector::create(params.clone())?;

        Ok(Self {
            params_standard: params.clone(),
            params_relaxed: params.clone(),
            params_super_relaxed: params,

            standard,
            relaxed,
            super_relaxed,
            keypoints: Vector::<opencv::core::KeyPoint>::new(),
        })
    }

    pub fn new() -> Result<Self> {
        let params_standard = Self::blob_params_standard();
        let params_relaxed = Self::blob_params_relaxed();
        let params_super_relaxed = Self::blob_params_super_relaxed();

        let standard = SimpleBlobDetector::create(params_standard.clone())?;
        let relaxed = SimpleBlobDetector::create(params_relaxed.clone())?;
        let super_relaxed = SimpleBlobDetector::create(params_super_relaxed.clone())?;

        // let standard = SimpleBlobDetector::create(Self::blob_params_standard())?;

        // let p2 = SimpleBlobDetector_Params::default().unwrap();
        // debug!("max_inertia_ratio: {}", p2.max_inertia_ratio);

        Ok(Self {
            params_standard,
            params_relaxed,
            params_super_relaxed,

            standard,
            relaxed,
            super_relaxed,
            keypoints: Vector::<opencv::core::KeyPoint>::new(),
        })
    }

    pub fn make_clone(&self) -> Result<Self> {
        let standard = SimpleBlobDetector::create(self.params_standard.clone())?;
        let relaxed = SimpleBlobDetector::create(self.params_relaxed.clone())?;
        let super_relaxed = SimpleBlobDetector::create(self.params_super_relaxed.clone())?;

        Ok(Self {
            params_standard: self.params_standard.clone(),
            params_relaxed: self.params_relaxed.clone(),
            params_super_relaxed: self.params_super_relaxed.clone(),

            standard,
            relaxed,
            super_relaxed,
            keypoints: Vector::<opencv::core::KeyPoint>::new(),
        })
    }
}
