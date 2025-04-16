use egui::Response;
use num::{CheckedAdd, CheckedSub, Float};

pub fn make_scrollable_f<T>(
    ui: &mut egui::Ui,
    resp: Response,
    val: &mut T,
    increment: T,
    min: T,
    max: T,
) where
    T: Copy + PartialOrd + Float,
{
    if resp.hovered() {
        let delta = ui.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::MouseWheel {
                    unit: _,
                    delta,
                    modifiers,
                } => Some(*delta),
                _ => None,
            })
        });
        if let Some(delta) = delta {
            if delta.y > 0. {
                *val = *val + increment;
                *val = val.min(max);
            } else if delta.y < 0. {
                *val = *val - increment;
                *val = val.max(min);
            } else {
                // if let Some(x) = val.checked_sub(&increment) {
                //     *val = x;
                // }
            }
        }
    }
}

pub fn make_scrollable<T>(
    ui: &mut egui::Ui,
    resp: Response,
    //
    val: &mut T,
    increment: T,
    // min: Option<T>,
) where
    T: Copy + PartialOrd + CheckedAdd + CheckedSub,
{
    if resp.hovered() {
        let delta = ui.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::MouseWheel {
                    unit: _,
                    delta,
                    modifiers,
                } => Some(*delta),
                _ => None,
            })
        });
        if let Some(delta) = delta {
            if delta.y > 0. {
                *val = *val + increment;
            } else {
                if let Some(x) = val.checked_sub(&increment) {
                    *val = x;
                }
            }
        }
    }
}
