use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use opencv::{
    core::{Point, Ptr, Size, Vec3f, Vector},
    features2d::{SimpleBlobDetector, SimpleBlobDetector_Params},
    imgproc::{self, cvt_color, gaussian_blur, hough_circles, threshold, ThresholdTypes},
    prelude::*,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PreprocessStep {
    pub step: PreprocessStepType,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PreprocessStepType {
    ConvertGrayscale,
    ConvertLuma,
    GaussianBlur {
        ksize: u32,
        sigma: f64,
    },
    Threshold {
        threshold: f64,
        threshold_type: ThresholdType,
    },
    AdaptiveThreshold,
}

impl PreprocessStepType {
    pub fn to_str(&self) -> &str {
        match self {
            PreprocessStepType::ConvertGrayscale => "Convert Grayscale",
            PreprocessStepType::ConvertLuma => "Convert Luma",
            PreprocessStepType::GaussianBlur { .. } => "Gaussian Blur",
            PreprocessStepType::Threshold { .. } => "Threshold",
            PreprocessStepType::AdaptiveThreshold => "Adaptive Threshold",
        }
    }

    pub fn apply(&self, img: &Mat, img2: &mut Mat) -> Result<()> {
        match self {
            PreprocessStepType::ConvertGrayscale => todo!(),
            PreprocessStepType::ConvertLuma => {
                cvt_color(
                    &img,
                    img2,
                    opencv::imgproc::COLOR_RGB2YUV,
                    0,
                    opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
                )?;
            }
            PreprocessStepType::GaussianBlur { ksize, sigma } => todo!(),
            PreprocessStepType::Threshold {
                threshold,
                threshold_type,
            } => todo!(),
            PreprocessStepType::AdaptiveThreshold => todo!(),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ThresholdType {
    Binary,
    BinaryInv,
    BinaryTriangle,
    BinaryInvTriangle,
    BinaryOtsu,
    BinaryInvOtsu,
}

impl Default for PreprocessStepType {
    fn default() -> Self {
        PreprocessStepType::ConvertLuma
    }
}
