use std::sync::{Arc, Mutex};

use anyhow::{anyhow, bail, ensure, Context, Result};
use opencv::features2d::SimpleBlobDetector_Params;
use tracing::{debug, error, info, trace, warn};

use argmin::{
    core::{CostFunction, Error, Executor, Gradient, Hessian, Operator},
    solver::{
        neldermead::NelderMead,
        simulatedannealing::{Anneal, SimulatedAnnealing},
    },
};
use argmin_observer_slog::SlogLogger;
use ndarray::{array, Array1};
use rand::prelude::*;
use rand_xoshiro::{rand_core::SeedableRng, Xoshiro256PlusPlus};

use crate::{
    ui::data_labeling::SavedTargets,
    vision::{blob_detection::BlobDetectors, VisionSettings},
};

#[derive(Debug)]
pub struct OptimizeData {
    pub images: Vec<(
        String,
        ((f32, f32), image::ImageBuffer<image::Rgb<u8>, Vec<u8>>),
    )>,

    pub detectors: BlobDetectors,

    pub params: SimpleBlobDetector_Params,

    pub vision_params: VisionSettings,

    rng: Arc<Mutex<Xoshiro256PlusPlus>>,
}

impl OptimizeData {
    #[cfg(feature = "nope")]
    pub const PARAM_RANGES: [(i32, i32); 3] = [
        //  (1, 254)
        (3, 15),
        (-100, 100),
        (1, 254),
    ];

    pub const PARAM_RANGES: [(f32, f32); 7] = [
        /// min_threshold
        (1., 100.),
        /// max_threshold
        (50., 250.),
        // /// threshold_step
        // (1., 10.),
        /// min_area
        (1000., 15_000.),
        /// max_area
        (2000., 50_000.),
        /// min_circularity
        (0.2, 0.8),
        /// min_convexity
        (0.2, 0.8),
        /// min_inertia_ratio
        (0.2, 0.8),
    ];
}

impl CostFunction for OptimizeData {
    // type Param = Vec<i32>;
    type Param = Vec<f32>;

    type Output = f32;

    fn cost(&self, param: &Self::Param) -> std::result::Result<Self::Output, Error> {
        let mut ps = self.params.clone();
        Self::apply_params(&mut ps, param);
        self.evaluate(&ps)
    }
}

#[cfg(feature = "nope")]
impl Operator for OptimizeData {
    // type Param = Vec<i32>;
    type Param = Array1<f64>;
    type Output = f64;

    fn apply(&self, param: &Self::Param) -> std::result::Result<Self::Output, Error> {
        let mut ps = self.params.clone();
        Self::apply_params(&mut ps, param);
        self.evaluate(&ps)
    }
}

// #[cfg(feature = "nope")]
impl Anneal for OptimizeData {
    // type Param = Vec<i32>;
    // type Output = Vec<i32>;
    type Param = Vec<f32>;
    type Output = Vec<f32>;
    type Float = f32;

    fn anneal(
        &self,
        param: &Self::Param,
        extent: Self::Float,
    ) -> std::result::Result<Self::Output, Error> {
        let mut rng = self.rng.lock().unwrap();
        let mut new_params: Self::Output = param
            .iter()
            .enumerate()
            .map(|(i, &current_val)| {
                let (min, max) = Self::PARAM_RANGES[i];
                // Calculate perturbation as a fraction of the parameter's range, scaled by extent
                let range = max - min;
                let step = range * extent; // Base step size on range and temperature
                let perturbation = rng.random_range(-step..=step);
                let new_val = current_val + perturbation;
                // Clamp to ensure within valid bounds
                new_val.clamp(min, max)
            })
            .collect();

        // // Ensure min_threshold is less than max_threshold
        // new_params[1] = (new_params[0] + new_params[2] + 1.0).max(new_params[1]);

        // // Ensure min_area is less than max_area
        // new_params[4] = (new_params[3] + 1.0).max(new_params[3]);

        new_params[1] = (new_params[0] + 2.0).max(new_params[1]);

        new_params[3] = (new_params[2] + 1.0).max(new_params[3]);

        Ok(new_params)
    }

    #[cfg(feature = "nope")]
    fn anneal(
        &self,
        param: &Self::Param,
        extent: Self::Float,
    ) -> std::result::Result<Self::Output, Error> {
        let mut ps = self.params.clone();

        let mut rng = self.rng.lock().unwrap();

        let mut new_param = param.clone();

        for i in 0..param.len() {
            let bounds = Self::PARAM_RANGES[i];

            let mut new_val = param[i] as f64 + extent * rng.random::<f64>() * 2. - extent;

            if new_val < bounds.0 as f64 {
                new_val = bounds.0 as f64;
            } else if new_val > bounds.1 as f64 {
                new_val = bounds.1 as f64;
            }

            new_param[i] = new_val as i32;
        }

        Ok(new_param)
    }
}

/// Load
impl OptimizeData {
    pub fn load() -> Result<Self> {
        let mut saved_targets = {
            let path = "test_images/saved_targets.toml";
            if let Ok(s) = std::fs::read_to_string(&path) {
                let saved_targets: SavedTargets = toml::from_str(&s)?;
                saved_targets
            } else {
                SavedTargets::default()
            }
        };

        let mut images = Vec::new();

        for (path, target) in saved_targets.targets.iter() {
            let mut image = image::open(path)?.into_rgb8();
            images.push((
                path.to_string_lossy().to_string(),
                ((target.0 as f32, target.1 as f32), image),
            ));
        }

        let detectors = BlobDetectors::new()?;

        Ok(Self {
            images,
            params: detectors.params_standard.clone(),
            detectors,
            vision_params: VisionSettings::default(),
            rng: Arc::new(Mutex::new(Xoshiro256PlusPlus::seed_from_u64(1234))),
        })
    }
}

/// Blob Detector
impl OptimizeData {
    pub fn apply_params(blob_params: &mut SimpleBlobDetector_Params, params: &Vec<f32>) {
        debug!("Applying params: {:?}", params);
        blob_params.min_threshold = params[0];
        blob_params.max_threshold = params[1];
        // blob_params.threshold_step = params[2];
        blob_params.min_area = params[2];
        blob_params.max_area = params[3];
        blob_params.min_circularity = params[4];
        blob_params.min_convexity = params[5];
        blob_params.min_inertia_ratio = params[6];
    }

    pub fn evaluate(&self, settings: &SimpleBlobDetector_Params) -> Result<f32> {
        // let mut detectors = self.detectors.make_clone().unwrap();
        let mut detectors = BlobDetectors::new_with_params(settings.clone())?;

        let mut errors: Vec<(f32, f32)> = vec![];
        let mut misses: usize = 0;

        let mut debug_output = true;
        // let mut debug_output = false;

        let output_dir = "test_images/output";
        if debug_output {
            if !std::path::Path::new(output_dir).exists() {
                std::fs::create_dir(output_dir)?;
            }
        }

        // debug!("blur_kernel_size: {}", settings.blur_kernel_size);
        // debug!("blur_sigma: {}", settings.blur_sigma);
        // debug!("threshold_block_size: {}", settings.threshold_block_size);

        // debug!("Skipping all but first");

        for (path, (target, img)) in self.images.iter() {
            let (mat, result) = match crate::vision::locate_nozzle::locate_nozzle(
                &img,
                &self.vision_params,
                &mut detectors,
            ) {
                Err(e) => {
                    // error!("Failed to locate nozzle in image {}: {}", path, e);
                    error!("Failed to locate nozzle: {}", e);
                    continue;
                }
                Ok(result) => {
                    if debug_output {
                        let output_path = {
                            let path = std::path::Path::new(path);
                            let base = path.parent().unwrap();
                            let output_path = path.file_stem().unwrap();
                            base.join(std::path::Path::new("output"))
                                .join(std::path::Path::new(&format!(
                                    "{}_output.jpg",
                                    output_path.to_string_lossy()
                                )))
                        };

                        debug!("Saving output to {:?}", output_path);
                        opencv::imgcodecs::imwrite(
                            output_path.to_str().unwrap(),
                            &result.0,
                            &opencv::core::Vector::new(),
                        )
                        .unwrap();
                    }

                    //
                    result
                }
            };

            if let Some((x, y, radius)) = result {
                // let x = x as f32 / self.vision_params.prescale as f32;
                // let y = y as f32 / self.vision_params.prescale as f32;

                let (x, y) = (x as f32, y as f32);

                debug!("X: {:.4}, Y: {:.4}", x, y);

                // let (x, y) = (x as f32, y as f32);
                let error_x = target.0 - x;
                let error_y = target.1 - y;

                let error_x = target.0 - x;
                let error_y = target.1 - y;

                errors.push((error_x, error_y));
            } else {
                misses += 1;
            }

            debug!("breaking early");
            break;
        }

        let mut total_error = (0.0, 0.0);
        let mut error_sq = (0., 0.);

        let mut total_dist_err = 0.0;

        for (error_x, error_y) in errors.iter() {
            total_error.0 += error_x.abs();
            total_error.1 += error_y.abs();

            error_sq.0 += error_x.powi(2);
            error_sq.1 += error_y.powi(2);

            total_dist_err += (error_x.powi(2) + error_y.powi(2)).sqrt();
        }

        let avg_error = (
            total_error.0 / errors.len() as f32,
            total_error.1 / errors.len() as f32,
        );

        debug!("Errors: {:?}", errors.len());
        debug!("Misses: {:?}", misses);
        debug!("Average Error: {:.3}, {:.3}", avg_error.0, avg_error.1);

        let mut average_dist_err = total_dist_err / errors.len() as f32;

        for _ in 0..misses {
            average_dist_err += 1000.0;
        }

        if average_dist_err.is_nan() {
            return Ok(1e30);
        }

        // Ok((avg_error.0 + avg_error.1) / 2.)
        // Ok(error_sq.0 + error_sq.1)
        Ok(average_dist_err)
    }

    // #[cfg(feature = "nope")]
    pub fn optimize() -> Result<()> {
        debug!("Optimizing...");

        let mut data = OptimizeData::load().unwrap();

        data.vision_params.prescale = 2.0;

        let t0 = std::time::Instant::now();

        let mut init_guess: Vec<f32> = OptimizeData::PARAM_RANGES
            .iter()
            .map(|(min, max)| (*min as f32 + *max as f32) / 2.)
            .collect();

        init_guess[0] = 50.;
        init_guess[1] = 100.;
        // init_guess[1] = init_guess[0];
        // init_guess[2] = 1.;
        init_guess[2] = 1000.;
        init_guess[3] = 50_000.;
        init_guess[4] = 0.75;
        init_guess[5] = 0.8;
        init_guess[6] = 0.2;

        data.cost(&init_guess).unwrap();

        let t1 = std::time::Instant::now();

        debug!("Time: {:.1}", t1.duration_since(t0).as_micros() / 1_000);

        Ok(())
    }

    #[cfg(feature = "nope")]
    /// Simulated Annealing optimization
    pub fn optimize() -> Result<()> {
        debug!("Optimizing...");

        let data = OptimizeData::load().unwrap();

        let init_guess: Vec<f32> = OptimizeData::PARAM_RANGES
            .iter()
            .map(|(min, max)| (*min as f32 + *max as f32) / 2.)
            .collect();

        let solver = SimulatedAnnealing::new(15.0)?
            // Optional: Define temperature function (defaults to `SATempFunc::TemperatureFast`)
            .with_temp_func(argmin::solver::simulatedannealing::SATempFunc::Boltzmann);

        let executor = Executor::new(data, solver)
            .configure(|state| {
                state
                    .param(init_guess.clone())
                    .max_iters(1000)
                    .target_cost(0.0)
            })
            .add_observer(
                SlogLogger::term(),
                argmin::core::observers::ObserverMode::Always,
            );

        debug!("Starting optimization...");
        let result = executor.run().unwrap();

        debug!("Done");

        debug!("Result: {}", result);

        Ok(())
    }

    /// Nelder-Mead optimization
    #[cfg(feature = "nope")]
    pub fn optimize() -> Result<()> {
        debug!("Optimizing...");

        let data = OptimizeData::load().unwrap();

        let init_guess: Vec<f64> = OptimizeData::PARAM_RANGES
            .iter()
            .map(|(min, max)| (*min as f64 + *max as f64) / 2.)
            .collect();

        let mut simplex = vec![init_guess.clone()];

        for i in 0..init_guess.len() {
            let mut perturbed_point = init_guess.clone();

            let k = OptimizeData::PARAM_RANGES[i];
            let k = (k.0 + k.1) / 10.;

            perturbed_point[i] += k; // Perturb the i-th parameter by 1/10th of the range

            simplex.push(perturbed_point);
        }

        let nelder_mead = NelderMead::new(simplex).with_sd_tolerance(0.0001)?;

        let executor = Executor::new(data, nelder_mead)
            .configure(|state| {
                state
                    .param(init_guess.clone())
                    .max_iters(100)
                    .target_cost(0.0)
            })
            .add_observer(
                SlogLogger::term(),
                argmin::core::observers::ObserverMode::Always,
            );

        debug!("Starting optimization...");
        executor.run().unwrap();

        debug!("Done");

        Ok(())
    }
}

/// Preprocess
#[cfg(feature = "nope")]
impl OptimizeData {
    pub fn apply_params(vision_params: &mut VisionSettings, params: &Vec<i32>) {
        vision_params.blur_kernel_size = params[0] as u32;
        vision_params.blur_sigma = params[1] as f64 / 10.;
        vision_params.threshold_block_size = params[2] as u32;
    }

    pub fn evaluate(&self, settings: &VisionSettings) -> Result<f64> {
        let mut detectors = self.detectors.make_clone().unwrap();

        let mut errors: Vec<(f64, f64)> = vec![];
        let mut misses: Vec<String> = vec![];

        debug!("blur_kernel_size: {}", settings.blur_kernel_size);
        debug!("blur_sigma: {}", settings.blur_sigma);
        debug!("threshold_block_size: {}", settings.threshold_block_size);

        // debug!("Skipping all but first");

        for (_, (target, img)) in self.images.iter() {
            let (mat, result) = match crate::vision::locate_nozzle::locate_nozzle(
                &img,
                &settings,
                &mut detectors,
            ) {
                Err(e) => {
                    // error!("Failed to locate nozzle in image {}: {}", path, e);
                    error!("Failed to locate nozzle: {}", e);
                    continue;
                }
                Ok(result) => result,
            };

            if let Some((x, y, radius)) = result {
                let error_x = target.0 - x;
                let error_y = target.1 - y;

                let error_x = target.0 - x;
                let error_y = target.1 - y;

                errors.push((error_x, error_y));
            }
        }

        let mut total_error = (0.0, 0.0);
        let mut error_sq = (0., 0.);

        for (error_x, error_y) in errors.iter() {
            total_error.0 += error_x.abs();
            total_error.1 += error_y.abs();

            error_sq.0 += error_x.powi(2);
            error_sq.1 += error_y.powi(2);
        }

        let avg_error = (
            total_error.0 / errors.len() as f64,
            total_error.1 / errors.len() as f64,
        );

        debug!("Average Error: {:.1}, {:.1}", avg_error.0, avg_error.1);

        // Ok((avg_error.0 + avg_error.1) / 2.)
        Ok(error_sq.0 + error_sq.1)
    }

    pub fn optimize() -> Result<()> {
        debug!("Optimizing...");

        let data = OptimizeData::load().unwrap();

        // let nelder_mead = NelderMead::new(vec![7, 60, 215]);
        let solver = SimulatedAnnealing::new(1_000.)?
            .with_temp_func(argmin::solver::simulatedannealing::SATempFunc::TemperatureFast)
            // .with_stall_accepted(iter)
            // .with_stall_best(iter)
            ;

        let init_param: Vec<i32> = vec![7, 60, 215];

        let executor = Executor::new(data, solver)
            .configure(|state| {
                state
                    .param(init_param)
                    // .param(init_param)
                    .max_iters(100)
                    .target_cost(0.0)
            })
            .add_observer(
                SlogLogger::term(),
                argmin::core::observers::ObserverMode::Always,
            );

        debug!("Starting optimization...");
        executor.run().unwrap();

        debug!("Done");

        Ok(())
    }
}
