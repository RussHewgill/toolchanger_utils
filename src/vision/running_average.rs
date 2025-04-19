use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::collections::VecDeque;
use std::time::Instant;

use crate::{
    ui::{
        auto_offset::AutoOffsetType,
        ui_types::{App, Axis},
    },
    vision::VisionSettings,
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CircleAggregator {
    window_size: usize,
    min_samples: usize,
    // high_threshold: f64,
    // low_threshold: f64,
    sum: (f64, f64, f64),
    sum_sq: (f64, f64, f64),
    buffer: VecDeque<Option<(f64, f64, f64)>>,
    valid: usize,
}

impl Default for CircleAggregator {
    fn default() -> Self {
        Self {
            // window_size: 30,
            window_size: 45,
            min_samples: 5,
            // high_threshold: 0.8,
            // low_threshold: 0.2,
            sum: (0., 0., 0.),
            sum_sq: (0., 0., 0.),
            buffer: VecDeque::with_capacity(120),
            valid: 0,
        }
    }
}

impl CircleAggregator {
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.sum = (0., 0., 0.);
        self.sum_sq = (0., 0., 0.);
        self.valid = 0;
    }

    pub fn add_frame(&mut self, pos: Option<(f64, f64, f64)>) {
        // debug!("Adding frame: {:?}", pos);
        if self.buffer.len() > self.window_size {
            panic!("Buffer overflow: more than {} elements", self.window_size);
        } else if self.buffer.len() == self.window_size {
            if let Some(old_pos) = self.buffer.pop_front() {
                if let Some((x, y, r)) = old_pos {
                    self.sum.0 -= x;
                    self.sum.1 -= y;
                    self.sum.2 -= r;
                    self.sum_sq.0 -= x.powi(2);
                    self.sum_sq.1 -= y.powi(2);
                    self.sum_sq.2 -= r.powi(2);
                    self.valid -= 1;
                }
            }
        }

        self.buffer.push_back(pos);
        if let Some(pos) = pos {
            self.sum.0 += pos.0;
            self.sum.1 += pos.1;
            self.sum.2 += pos.2;
            self.sum_sq.0 += pos.0.powi(2);
            self.sum_sq.1 += pos.1.powi(2);
            self.sum_sq.2 += pos.2.powi(2);
            self.valid += 1;
        }
    }

    /// Returns the current best guess as the average of tracked points
    pub fn current_guess(&self) -> Option<(f64, f64, f64)> {
        if self.buffer.is_empty() || self.valid < self.min_samples {
            return None;
        }

        // let n = self.buffer.len() as f64;

        let n = self.valid as f64;

        Some((self.sum.0 / n, self.sum.1 / n, self.sum.2 / n))
    }

    pub fn confidence(&self) -> Option<(f64, (f64, f64, f64))> {
        if self.buffer.is_empty() {
            // warn!("Buffer is empty, cannot calculate confidence");
            return None;
        }

        if self.valid < self.min_samples {
            // warn!("Not enough valid samples to calculate confidence");
            return None;
        }

        let detection_rate = self.valid as f64 / self.buffer.len() as f64;

        // Apply a sigmoid-like scaling to the detection rate
        // This makes a few Nones only slightly decrease confidence,
        // while many Nones cause a more dramatic decrease

        let detection_factor = if self.valid == self.buffer.len() {
            1.0
        } else {
            1.0 / (1.0 + (-20.0 * (detection_rate - 0.8)).exp())
        };

        // let n = self.buffer.len() as f64;
        let n = self.valid as f64;

        let mean = (self.sum.0 / n, self.sum.1 / n, self.sum.2 / n);

        // Calculate variance, ensuring non-negative values
        let var_x = (self.sum_sq.0 / n - mean.0.powi(2)).max(0.0);
        let var_y = (self.sum_sq.1 / n - mean.1.powi(2)).max(0.0);
        let var_r = (self.sum_sq.2 / n - mean.2.powi(2)).max(0.0);

        // debug!("Variance: ({:.2}, {:.2}, {:.2})", var_x, var_y, var_r);

        // Calculate standard errors
        let std_err_x = var_x.sqrt() / n.sqrt();
        let std_err_y = var_y.sqrt() / n.sqrt();
        let std_err_r = var_r.sqrt() / n.sqrt();

        // debug!(
        //     "Standard errors: ({:.2}, {:.2}, {:.2})",
        //     std_err_x, std_err_y, std_err_r
        // );

        // Average standard error and calculate confidence
        let avg_std_err = (std_err_x + std_err_y) / 2.0;

        let consistency_x = 1.0 / (1.0 + std_err_x);
        let consistency_y = 1.0 / (1.0 + std_err_y);
        let consistency_r = 1.0 / (1.0 + std_err_r);

        // Average consistency for X and Y (ignoring R for overall confidence)
        let avg_consistency = (consistency_x + consistency_y) / 2.0;

        // debug!(
        //     "Confidence: ({:.2}, {:.2}, {:.2})",
        //     confidence_x, confidence_y, confidence_r
        // );

        let confidence = 1.0 / (1.0 + avg_std_err);
        // debug!("Confidence: {:.2}", confidence);

        // Weight detection rate and consistency contributions to final confidence
        // (80% consistency, 20% detection rate)
        let weight_consistency = 0.8;
        let weight_detection = 0.2;

        // Calculate final confidence values
        let confidence_x = consistency_x * weight_consistency + detection_factor * weight_detection;
        let confidence_y = consistency_y * weight_consistency + detection_factor * weight_detection;
        let confidence_r = consistency_r * weight_consistency + detection_factor * weight_detection;
        let confidence = avg_consistency * weight_consistency + detection_factor * weight_detection;

        Some((confidence, (confidence_x, confidence_y, confidence_r)))
    }
}
