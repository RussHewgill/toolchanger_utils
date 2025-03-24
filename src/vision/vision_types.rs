// use egui_struct::EguiStruct;

use std::collections::VecDeque;

// pub use self::running_average::*;
pub use self::circle_aggregator::*;

#[derive(Debug, PartialEq)]
pub enum NozzlePosition {
    Centered,
    Up,
    Down,
    Left,
    Right,
    NotVisible,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum WebcamCommand {
    SaveScreenshot,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum WebcamMessage {
    /// X, Y, radius
    FoundNozzle((f64, f64, f64)),
    NozzleNotFound,
}

// #[cfg(feature = "nope")]
pub mod circle_aggregator {
    use std::collections::VecDeque;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct CircleAggregator {
        window_size: usize,
        high_threshold: f64,
        low_threshold: f64,
        buffer: VecDeque<Option<(f64, f64, f64)>>,
    }

    impl Default for CircleAggregator {
        fn default() -> Self {
            Self {
                window_size: 15,
                high_threshold: 0.8,
                low_threshold: 0.2,
                buffer: VecDeque::with_capacity(30),
            }
        }
    }

    impl CircleAggregator {
        pub fn new(window_size: usize, high_threshold: f64, low_threshold: f64) -> Self {
            Self {
                window_size,
                high_threshold,
                low_threshold,
                buffer: VecDeque::with_capacity(window_size),
            }
        }

        pub fn clear(&mut self) {
            self.buffer.clear();
        }

        pub fn add_frame(&mut self, circle: Option<(f64, f64, f64)>) {
            if self.buffer.len() >= self.window_size {
                self.buffer.pop_front();
            }
            self.buffer.push_back(circle);
        }

        pub fn calculate_median(hits: &[&(f64, f64, f64)]) -> Option<(f64, f64, f64)> {
            if hits.is_empty() {
                return None;
            }

            let mut xs: Vec<f64> = hits.iter().map(|c| c.0).collect();
            let mut ys: Vec<f64> = hits.iter().map(|c| c.1).collect();
            let mut radii: Vec<f64> = hits.iter().map(|c| c.2).collect();

            xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            radii.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let mid = hits.len() / 2;
            Some((xs[mid], ys[mid], radii[mid]))
        }

        pub fn get_result(&self) -> (f64, Option<(f64, f64, f64)>) {
            let total = self.buffer.len();
            if total == 0 {
                return (0.0, None);
            }

            let hits: Vec<&(f64, f64, f64)> =
                self.buffer.iter().filter_map(|c| c.as_ref()).collect();

            // tracing::debug!("hits: {:?}", hits.len());
            // tracing::debug!("total: {:?}", total);

            let confidence = hits.len() as f64 / total as f64;

            if confidence >= self.high_threshold {
                let median_circle = Self::calculate_median(&hits);
                (confidence, median_circle)
            } else if confidence <= self.low_threshold {
                (confidence, None)
            } else {
                (confidence, None)
            }
        }

        /// Calculates the margin of error for the current circle detections.
        /// Returns (x_error, y_error, radius_error) as standard deviations.
        /// Returns None if there are no hits or only one hit (can't calculate deviation).
        pub fn calculate_margin_of_error(&self) -> Option<((f64, f64, f64), (f64, f64, f64))> {
            let hits: Vec<&(f64, f64, f64)> =
                self.buffer.iter().filter_map(|c| c.as_ref()).collect();

            if hits.len() <= 5 {
                return None;
            }

            // Get median values
            let median = Self::calculate_median(&hits)?;

            // Calculate sum of squared differences from median
            let (sum_sq_x, sum_sq_y, sum_sq_r) =
                hits.iter().fold((0.0, 0.0, 0.0), |acc, &&(x, y, r)| {
                    let dx = x - median.0;
                    let dy = y - median.1;
                    let dr = r - median.2;

                    (acc.0 + dx * dx, acc.1 + dy * dy, acc.2 + dr * dr)
                });

            // Calculate standard deviation
            let n = hits.len() as f64;
            let std_dev_x = (sum_sq_x / n).sqrt();
            let std_dev_y = (sum_sq_y / n).sqrt();
            let std_dev_r = (sum_sq_r / n).sqrt();

            Some((median, (std_dev_x, std_dev_y, std_dev_r)))
        }
    }
}

#[cfg(feature = "nope")]
pub mod running_average {
    use std::collections::VecDeque;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct RunningAverage {
        positions: VecDeque<(f64, f64)>,
        sum: (f64, f64),
        sum_squared: (f64, f64),
        max_length: usize,
    }

    impl Default for RunningAverage {
        fn default() -> Self {
            Self::new()
        }
    }

    impl RunningAverage {
        pub fn new() -> Self {
            Self {
                positions: VecDeque::new(),
                sum: (0.0, 0.0),
                sum_squared: (0.0, 0.0),
                max_length: 20,
            }
        }

        pub fn clear(&mut self) {
            self.positions.clear();
            self.sum = (0.0, 0.0);
            self.sum_squared = (0.0, 0.0);
        }

        pub fn push_position(&mut self, pos: (f64, f64)) {
            if self.positions.len() == self.max_length {
                if let Some((old_x, old_y)) = self.positions.pop_front() {
                    self.sum.0 -= old_x;
                    self.sum.1 -= old_y;
                    self.sum_squared.0 -= old_x * old_x;
                    self.sum_squared.1 -= old_y * old_y;
                }
            }

            self.positions.push_back(pos);
            self.sum.0 += pos.0;
            self.sum.1 += pos.1;
            self.sum_squared.0 += pos.0 * pos.0;
            self.sum_squared.1 += pos.1 * pos.1;
        }

        pub fn calculate_variance(&self) -> (f64, f64) {
            let n = self.positions.len() as f64;
            if n == 0.0 {
                return (0.0, 0.0);
            }

            let mean_x = self.sum.0 / n;
            let mean_y = self.sum.1 / n;

            let variance_x = (self.sum_squared.0 / n) - (mean_x * mean_x);
            let variance_y = (self.sum_squared.1 / n) - (mean_y * mean_y);

            (variance_x.max(0.0), variance_y.max(0.0))
        }

        pub fn calculate_confidence(&self) -> f64 {
            let (variance_x, variance_y) = self.calculate_variance();

            let total_variance = variance_x + variance_y;

            // Simple confidence function - adjust as needed
            1.0 / (1.0 + total_variance.sqrt())

            // let confidence_x = 1.0 - (variance_x / (self.sum.0 * self.sum.0));
            // let confidence_y = 1.0 - (variance_y / (self.sum.1 * self.sum.1));

            // (confidence_x.max(0.0), confidence_y.max(0.0))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WebcamSettings {
    // pub camera_index: usize,
    pub filter_step: usize,
    // pub threshold: i32,
    pub blur_kernel_size: i32,
    pub blur_sigma: f64,
    pub adaptive_threshold: bool,
    pub threshold_block_size: i32,
    // pub adaptive_threshold_c: i32,
    /// 0: Binary Inv, 1: Binary Inv + Triangle, 2: Binary Inv + Otsu
    pub threshold_type: usize,
    pub use_hough: bool,
    pub draw_circle: bool,
    pub crosshair_size: f32,
    pub pixels_per_mm: f64,
    // pub mirror: (bool, bool),
    // pub rotate: usize,
    pub preprocess_pipeline: usize,
}

impl WebcamSettings {
    pub const NUM_FILTER_STEPS: usize = 4;
}

impl Default for WebcamSettings {
    fn default() -> Self {
        Self {
            // crosshair_size: 0.5,
            // camera_index: 0,
            filter_step: 0,
            // threshold: 35,
            blur_kernel_size: 7,
            blur_sigma: 6.0,
            adaptive_threshold: false,
            // threshold_block_size: 3,
            threshold_block_size: 17,
            // adaptive_threshold_c: 1,
            threshold_type: 1,
            use_hough: false,
            draw_circle: true,
            crosshair_size: 60.,
            pixels_per_mm: 200.,
            // mirror: (false, false),
            // rotate: 3,
            preprocess_pipeline: 0,
        }
    }
}
