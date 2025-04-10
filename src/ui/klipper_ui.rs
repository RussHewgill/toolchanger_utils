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
        let Some(s) = self.klipper_status.as_ref() else {
            debug!("klipper is not connected");
            return None;
        };

        let pos = s.blocking_read().position;

        pos
    }

    pub fn fetch_position(&mut self) -> Option<(f64, f64, f64)> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.send_klipper(KlipperCommand::GetPosition(tx));

        let Ok(pos) = rx.blocking_recv() else {
            error!("Failed to get position from klipper");
            return None;
        };

        pos
    }

    pub fn move_to_position(&mut self, pos: (f64, f64), bounce: bool) {
        self.send_klipper(KlipperCommand::MoveToPosition(
            (pos.0, pos.1, self.options.z_height),
            if bounce {
                Some(self.options.bounce_amount)
            } else {
                None
            },
        ));
    }

    pub fn disable_motors(&mut self) {
        self.send_klipper(KlipperCommand::DisableMotors);
    }

    #[cfg(feature = "nope")]
    pub fn move_relative(&mut self, amount: (f64, f64), bounce: bool) {
        todo!()
    }

    pub fn move_axis_relative(&mut self, axis: Axis, amount: f64, bounce: bool) {
        self.send_klipper(KlipperCommand::MoveAxisRelative(
            axis,
            amount,
            if bounce {
                Some(self.options.bounce_amount)
            } else {
                None
            },
        ));
    }

    pub fn dropoff_tool(&mut self) {
        self.send_klipper(KlipperCommand::DropTool);
        self.active_tool = None;
    }

    pub fn pickup_tool(&mut self, tool: i32, move_to_camera: bool) {
        if tool < 0 {
            error!("Invalid tool number: {}", tool);
            return;
        }

        self.send_klipper(KlipperCommand::PickTool(tool as u32));

        self.active_tool = Some(tool as usize);

        if move_to_camera {
            if let Some(pos) = self.camera_pos {
                self.move_to_position(pos, true);
            }
        }
    }

    pub fn adjust_offset_from_camera(&mut self, tool: usize, (x, y): (f64, f64)) {
        todo!()
    }
}
