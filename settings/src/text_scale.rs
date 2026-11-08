//! Desktop text scaling is separate from Wayland's monitor/device scale.
//! Like Strata, snap the 13px root once, then retain the relative type roles.

use std::{
    ffi::OsStr,
    path::Path,
    sync::mpsc::{self, Receiver},
    thread,
};

const ROOT_SIZE: f32 = 13.0;

fn valid(value: f32) -> bool {
    value.is_finite() && (0.5..=4.0).contains(&value)
}

fn parse(source: &str) -> Option<f32> {
    source.trim().parse::<f32>().ok().filter(|v| valid(*v))
}

pub struct DesktopTextScale {
    factor: f32,
    follow_desktop: bool,
    next_check: f64,
    task: Option<Receiver<Option<f32>>>,
}

impl DesktopTextScale {
    pub fn fixed(factor: f32) -> Self {
        Self {
            factor: if valid(factor) { factor } else { 1.0 },
            follow_desktop: false,
            next_check: 0.0,
            task: None,
        }
    }

    pub fn for_app() -> Self {
        if let Some(factor) = std::env::var("VOXTYPE_SETTINGS_TEXT_SCALE")
            .ok()
            .and_then(|value| parse(&value))
        {
            return Self::fixed(factor);
        }
        // Gallery captures/tests must not depend on the host's desktop settings.
        if cfg!(test) || rune::is_headless() {
            return Self::fixed(1.0);
        }
        let mut scale = Self {
            follow_desktop: true,
            ..Self::fixed(1.0)
        };
        scale.poll(0.0);
        scale
    }

    pub fn ratio(&self) -> f32 {
        (ROOT_SIZE * self.factor).round() / ROOT_SIZE
    }

    fn apply(&mut self, factor: Option<f32>) -> bool {
        let Some(factor) = factor.filter(|value| valid(*value)) else {
            return false; // Keep the last valid size if the desktop is unavailable.
        };
        let before = self.ratio();
        self.factor = factor;
        before != self.ratio()
    }

    /// Read-only, asynchronous and bounded by the shared command timeout. Never
    /// run a desktop subprocess in the renderer or modify a global preference.
    pub fn poll(&mut self, time: f64) -> bool {
        let mut changed = false;
        if let Some(task) = &self.task {
            match task.try_recv() {
                Ok(factor) => {
                    self.task = None;
                    changed = self.apply(factor);
                }
                Err(mpsc::TryRecvError::Disconnected) => self.task = None,
                Err(mpsc::TryRecvError::Empty) => (),
            }
        }
        if self.follow_desktop && self.task.is_none() && time >= self.next_check {
            let (sender, receiver) = mpsc::channel();
            self.task = Some(receiver);
            self.next_check = time + 2.0;
            thread::spawn(move || {
                let factor = crate::backend::command(
                    Path::new("gsettings"),
                    &[
                        OsStr::new("get"),
                        OsStr::new("org.gnome.desktop.interface"),
                        OsStr::new("text-scaling-factor"),
                    ],
                )
                .ok()
                .and_then(|value| parse(&value));
                let _ = sender.send(factor);
            });
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_stratas_snapped_root_independently_of_monitor_scale() {
        let scale = DesktopTextScale::fixed(0.8182);
        assert!((ROOT_SIZE * scale.ratio() - 11.0).abs() < 0.001);
        assert!((ROOT_SIZE * scale.ratio() * 1.25 - 13.75).abs() < 0.001);
        assert_eq!(DesktopTextScale::fixed(1.0).ratio(), 1.0);
        assert!((ROOT_SIZE * DesktopTextScale::fixed(1.25).ratio() - 16.0).abs() < 0.001);
    }

    #[test]
    fn invalid_values_preserve_the_last_good_size() {
        let mut scale = DesktopTextScale::fixed(0.8182);
        for source in ["", "NaN", "inf", "-1", "0", "999", "not a number"] {
            assert!(parse(source).is_none());
            assert!(!scale.apply(parse(source)));
            assert!((scale.ratio() - 11.0 / 13.0).abs() < 0.001);
        }
        assert!(scale.apply(parse("1.25\n")));
        assert!(
            !scale.apply(parse("1.251")),
            "same snapped root needs no redraw"
        );
    }

    #[test]
    fn background_scale_changes_invalidate_without_blocking() {
        let (sender, receiver) = mpsc::channel();
        let mut scale = DesktopTextScale::fixed(1.0);
        scale.task = Some(receiver);
        assert!(!scale.poll(0.0));
        sender.send(Some(0.8182)).unwrap();
        assert!(scale.poll(1.0));
        assert!((scale.ratio() - 11.0 / 13.0).abs() < 0.001);
        assert!(!scale.poll(2.0));
    }
}
