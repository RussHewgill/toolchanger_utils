use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::klipper_async::KlipperCommand;

use super::ui_types::*;

impl App {
    fn with_klipper<F>(&mut self, f: F)
    where
        F: FnOnce(&mut tokio::sync::mpsc::Sender<crate::klipper_async::KlipperCommand>),
    {
        let Some(klipper_tx) = &mut self.klipper_tx else {
            debug!("klipper is not connected");
            return;
        };
        f(klipper_tx)
    }

    fn with_klipper_val<F, S>(&mut self, f: F) -> Option<S>
    where
        F: FnOnce(&mut tokio::sync::mpsc::Sender<crate::klipper_async::KlipperCommand>) -> S,
    {
        let Some(klipper_tx) = &mut self.klipper_tx else {
            debug!("klipper is not connected");
            return None;
        };
        Some(f(klipper_tx))
    }

    fn send_klipper(&mut self, cmd: KlipperCommand) {
        self.with_klipper(|tx| {
            tx.blocking_send(cmd).unwrap_or_else(|e| {
                error!("Failed to send klipper command: {}", e);
            });
        });
    }

    pub fn home_all(&mut self) {
        self.send_klipper(KlipperCommand::HomeAll);
    }

    pub fn home_xy(&mut self) {
        self.send_klipper(KlipperCommand::HomeXY);
    }

    pub fn get_position(&mut self) -> Option<(f64, f64, f64)> {
        todo!()
    }

    pub fn fetch_position(&mut self) -> Option<(f64, f64, f64)> {
        todo!()
    }

    pub fn move_to_position(&mut self, pos: (f64, f64), bounce: bool) {
        todo!()
    }

    pub fn move_relative(&mut self, amount: (f64, f64), bounce: bool) {
        todo!()
    }

    pub fn move_axis_relative(&mut self, axis: Axis, amount: f64, bounce: bool) {
        todo!()
    }

    pub fn dropoff_tool(&mut self) {
        todo!()
    }

    pub fn pickup_tool(&mut self, tool: i32, move_to_camera: bool) {
        todo!()
    }

    pub fn adjust_offset_from_camera(&mut self, tool: usize, (x, y): (f64, f64)) {
        todo!()
    }
}
