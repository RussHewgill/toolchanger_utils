use std::sync::{Arc, Mutex};

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use argmin::{
    core::{CostFunction, Error, Executor, Gradient, Hessian, Operator},
    solver::simulatedannealing::{Anneal, SimulatedAnnealing},
};
use argmin_observer_slog::SlogLogger;
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
        ((f64, f64), image::ImageBuffer<image::Rgb<u8>, Vec<u8>>),
    )>,

    pub detectors: BlobDetectors,

    pub params: VisionSettings,

    rng: Arc<Mutex<Xoshiro256PlusPlus>>,
}

impl OptimizeData {
    pub const PARAM_RANGES: [(i32, i32); 3] = [
        //  (1, 254)
        (3, 15),
        (-100, 100),
        (1, 254),
    ];
}

impl CostFunction for OptimizeData {
    type Param = Vec<i32>;

    type Output = f64;

    fn cost(&self, param: &Self::Param) -> std::result::Result<Self::Output, Error> {
        let mut ps = self.params.clone();
        Self::apply_params(&mut ps, param);
        self.evaluate(&ps)
    }
}

impl Operator for OptimizeData {
    type Param = Vec<i32>;
    type Output = f64;

    fn apply(&self, param: &Self::Param) -> std::result::Result<Self::Output, Error> {
        let mut ps = self.params.clone();
        Self::apply_params(&mut ps, param);
        self.evaluate(&ps)
    }
}

impl Anneal for OptimizeData {
    type Param = Vec<i32>;
    type Output = Vec<i32>;
    type Float = f64;

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
            images.push((path.to_string_lossy().to_string(), (target.clone(), image)));
        }

        Ok(Self {
            images,
            detectors: BlobDetectors::new()?,
            params: VisionSettings::default(),
            rng: Arc::new(Mutex::new(Xoshiro256PlusPlus::seed_from_u64(1234))),
        })
    }

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
