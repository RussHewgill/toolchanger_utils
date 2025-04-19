use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::RichText;
use egui_extras::Column;
use std::time::Instant;

use crate::ui::ui_types::Axis;

use super::{auto_offset::AutoOffsetType, ui_types::App};

#[derive(Debug, Clone)]
pub struct AutoOffset {
    pub(super) auto_offset_type: AutoOffsetType,

    pub prev_position: (f64, f64),
    pub(super) last_move: Instant,
    pub(super) current_tool: i32,

    pub(super) check_repeatability: usize,

    pub(super) current_n: usize,

    /// (position, guessed offset from center)
    pub(super) repeatability: Vec<((f64, f64), (f64, f64))>,

    pub(super) offsets: Vec<Vec<((f64, f64), (f64, f64))>>,
}

// #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[derive(Debug, Clone)]
pub struct AutoOffsetSettings {
    pub target_max_offset: f64,
    // pub max_margin_of_error: f64,
    pub min_confidence_for_move: f64,
    pub min_interval_between_moves: f64,
    pub swap_axes: bool,
    pub mirror_axes: (bool, bool),
    pub resolution: f64,
    pub park_tool: bool,
    pub samples_per_tool: usize,
}

impl Default for AutoOffsetSettings {
    fn default() -> Self {
        AutoOffsetSettings {
            // target_max_offset: 0.01,
            target_max_offset: 0.00625,
            min_confidence_for_move: 0.95,
            min_interval_between_moves: 2.0,
            swap_axes: true,
            mirror_axes: (false, true),
            resolution: 0.00625,
            park_tool: true,
            samples_per_tool: 3,
            // samples_per_tool: 5,
        }
    }
}

impl Default for AutoOffset {
    fn default() -> Self {
        AutoOffset {
            auto_offset_type: AutoOffsetType::None,
            prev_position: (0.0, 0.0),
            last_move: Instant::now(),
            current_tool: -1,
            check_repeatability: 0,
            current_n: 0,
            repeatability: Vec::new(),
            offsets: Vec::new(),
        }
    }
}

impl AutoOffset {
    pub fn auto_offset_type(&self) -> AutoOffsetType {
        self.auto_offset_type
    }

    pub fn stop(&mut self) {
        // *self = Self::default();
        self.auto_offset_type = AutoOffsetType::None;
    }

    pub fn start_single(&mut self, pos: (f64, f64), tool: i32) {
        *self = Self::default();

        self.auto_offset_type = AutoOffsetType::SingleTool;
        self.prev_position = pos;
        self.current_tool = tool;
    }

    pub fn start_all_tools(&mut self, pos: (f64, f64), num_tools: usize) {
        *self = Self::default();

        self.auto_offset_type = AutoOffsetType::AllTools;
        self.prev_position = pos;
        self.current_tool = -1;

        self.offsets = vec![vec![]; num_tools];
    }

    pub fn start_repeatability(&mut self, pos: (f64, f64), tool: i32) {
        *self = Self::default();

        self.auto_offset_type = AutoOffsetType::RepeatabilityTest;
        self.prev_position = pos;
        self.current_tool = tool;
        self.check_repeatability = 10;
    }

    pub fn start_homing(&mut self, pos: (f64, f64), tool: i32) {
        *self = Self::default();

        self.auto_offset_type = AutoOffsetType::HomingTest;
        self.prev_position = pos;
        self.current_tool = tool;
        self.check_repeatability = 10;
    }

    pub fn process_repeatibility(&self, homing: bool) {
        debug!("Repeatability results:");

        let mut xs = self
            .repeatability
            .iter()
            // .map(|((x, _), _)| *x)
            .map(|((x, _), (offset, _))| *x + offset)
            .collect::<Vec<_>>();
        let mut ys = self
            .repeatability
            .iter()
            // .map(|((_, y), _)| *y)
            .map(|((_, y), (_, offset))| *y + offset)
            .collect::<Vec<_>>();

        /// calculate median:
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        ys.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let median_x = xs[xs.len() / 2];
        let median_y = ys[ys.len() / 2];

        debug!("Median: ({:.3}, {:.3})", median_x, median_y);

        // Calculate mean
        let mean_x = xs.iter().sum::<f64>() / xs.len() as f64;
        let mean_y = ys.iter().sum::<f64>() / ys.len() as f64;

        // Calculate standard deviation
        let variance_x = xs.iter().map(|&x| (x - mean_x).powi(2)).sum::<f64>() / xs.len() as f64;
        let variance_y = ys.iter().map(|&y| (y - mean_y).powi(2)).sum::<f64>() / ys.len() as f64;
        let std_dev_x = variance_x.sqrt();
        let std_dev_y = variance_y.sqrt();

        // Calculate min, max and range
        let min_x = *xs.first().unwrap();
        let max_x = *xs.last().unwrap();
        let min_y = *ys.first().unwrap();
        let max_y = *ys.last().unwrap();
        let range_x = max_x - min_x;
        let range_y = max_y - min_y;

        debug!("Median: ({:.3}, {:.3})", median_x, median_y);
        debug!("Mean: ({:.3}, {:.3})", mean_x, mean_y);
        debug!("Standard Deviation: ({:.3}, {:.3})", std_dev_x, std_dev_y);
        debug!("Min: ({:.3}, {:.3})", min_x, min_y);
        debug!("Max: ({:.3}, {:.3})", max_x, max_y);
        debug!("Range: ({:.3}, {:.3})", range_x, range_y);

        let output_dir = "data_output";
        if !std::path::Path::new(output_dir).exists() {
            std::fs::create_dir(output_dir).unwrap_or_else(|_| {
                error!("Failed to create output directory: {}", output_dir);
            });
        }

        let now = std::time::SystemTime::now();
        let now: chrono::DateTime<chrono::Utc> = now.into();
        let now = now.format("%Y-%m-%d_%H-%M-%S");

        let file_path = if homing {
            format!(
                "{}/repeatability_homing_tool_{}_data_{}.txt",
                output_dir, self.current_tool, now
            )
        } else {
            format!(
                "{}/repeatability_tool_{}_data_{}.txt",
                output_dir, self.current_tool, now
            )
        };

        match std::fs::File::create(&file_path) {
            Ok(mut file) => {
                use std::io::Write;

                // Write header
                if let Err(e) = writeln!(&mut file, "X,Y") {
                    error!("Failed to write header to file: {}", e);
                    return;
                }

                // Write each data point
                for ((x, y), (offset_x, offset_y)) in &self.repeatability {
                    if let Err(e) = writeln!(
                        &mut file,
                        "{:.6},{:.6},{:.6},{:.6}",
                        x, y, offset_x, offset_y
                    ) {
                        error!("Failed to write data point to file: {}", e);
                        return;
                    }
                }

                // Write statistics
                if let Err(e) = writeln!(&mut file, "\nStatistics: Tool {}", self.current_tool) {
                    error!("Failed to write to file: {}", e);
                    return;
                }

                let stats = [
                    // ("Median", median_x, median_y),
                    // ("Mean", mean_x, mean_y),
                    ("StdDev", std_dev_x, std_dev_y),
                    // ("Min", min_x, min_y),
                    // ("Max", max_x, max_y),
                    ("Range", range_x, range_y),
                ];

                for (name, x_val, y_val) in stats {
                    if let Err(e) = writeln!(&mut file, "{}: X={:.6}, Y={:.6}", name, x_val, y_val)
                    {
                        error!("Failed to write stats to file: {}", e);
                        return;
                    }
                }

                info!("Repeatability data saved to {}", file_path);
            }
            Err(e) => {
                error!("Failed to create file for repeatability data: {}", e);
            }
        }
    }

    pub fn repeatability_count_mut(&mut self) -> &mut usize {
        &mut self.check_repeatability
    }
}

impl App {
    pub fn process_offsets(&mut self) {
        warn!("TODO: process offsets");

        #[cfg(feature = "nope")]
        for tool in 0..self.options.num_tools {
            let offsets = self.auto_offset.offsets[tool as usize].clone();

            debug!("Tool {}:", tool);

            for (i, ((x, y), (offset_x, offset_y))) in offsets.iter().enumerate() {
                debug!(
                    "\tOffset[{}]: ({:.3}, {:.3}), ({:.3}, {:.3})",
                    i, x, y, offset_x, offset_y
                );
            }
        }

        let mut camera_pos = (0.0, 0.0);

        for tool in 0..self.options.num_tools {
            let offsets = self.auto_offset.offsets[tool as usize].clone();

            let mut xs = offsets
                .iter()
                .map(|((x, _), (offset, _))| *x + offset)
                .collect::<Vec<_>>();
            let mut ys = offsets
                .iter()
                .map(|((_, y), (_, offset))| *y + offset)
                .collect::<Vec<_>>();

            /// calculate median:
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            ys.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let median_x = xs[xs.len() / 2];
            let median_y = ys[ys.len() / 2];

            if tool == 0 {
                camera_pos = (median_x, median_y);
            } else {
                let offset_x = median_x - camera_pos.0;
                let offset_y = median_y - camera_pos.1;

                debug!(
                    "Setting tool {} offset: ({:.3}, {:.3})",
                    tool, offset_x, offset_y
                );

                self.set_tool_offset(tool, Axis::X, offset_x);
                self.set_tool_offset(tool, Axis::Y, offset_y);
            }
        }
    }
}
