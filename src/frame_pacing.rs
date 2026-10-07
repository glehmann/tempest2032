//! Detects late application updates. Actual display presentation timing is not
//! exposed by Bevy/wgpu, so these are possible missed refreshes, not confirmed drops.

use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;
use std::time::Duration;

#[derive(Default)]
pub(crate) struct FramePacing {
    refresh_rate: Option<u32>,
    next_warning: Duration,
    late_intervals: u32,
    worst: Duration,
}

impl FramePacing {
    fn sample(
        &mut self,
        now: Duration,
        delta: Duration,
        refresh_rate: Option<u32>,
    ) -> Option<(u32, Duration)> {
        let Some(rate) = refresh_rate.filter(|rate| *rate > 0) else {
            *self = Self::default();
            return None;
        };
        if self.refresh_rate != Some(rate) {
            *self = Self {
                refresh_rate: Some(rate),
                ..default()
            };
            return None;
        }
        // Half a refresh period of tolerance avoids classifying small scheduling
        // jitter as a missed refresh. This cannot detect every presentation miss.
        if delta.as_secs_f64() > 1500.0 / rate as f64 {
            self.late_intervals = self.late_intervals.saturating_add(1);
            self.worst = self.worst.max(delta);
        }
        if self.late_intervals == 0 || now < self.next_warning {
            return None;
        }
        self.next_warning = now + Duration::from_secs(1);
        Some((
            std::mem::take(&mut self.late_intervals),
            std::mem::take(&mut self.worst),
        ))
    }
}

pub(crate) fn warn_late_frames(
    time: Res<Time<Real>>,
    window: Option<Single<(Entity, &Window), With<PrimaryWindow>>>,
    mut pacing: Local<FramePacing>,
    _main_thread: NonSendMarker,
) {
    let Some(window) = window else {
        *pacing = default();
        return;
    };
    let (entity, window) = *window;
    if !window.focused || !window.visible {
        *pacing = default();
        return;
    }
    let Some((minimized, detected_rate)) = WINIT_WINDOWS.with_borrow(|windows| {
        windows.get_window(entity).map(|window| {
            (
                window.is_minimized() == Some(true),
                window
                    .current_monitor()
                    .and_then(|monitor| monitor.refresh_rate_millihertz())
                    .filter(|rate| *rate > 0),
            )
        })
    }) else {
        *pacing = default();
        return;
    };
    if minimized {
        *pacing = default();
        return;
    }
    let rate = detected_rate.unwrap_or(60_000);
    if pacing.refresh_rate != Some(rate) {
        if detected_rate.is_some() {
            info!("Frame pacing monitor: {:.3} Hz", rate as f64 / 1000.0);
        } else {
            info!("Monitor refresh rate unavailable; frame pacing warnings assume 60 Hz");
        }
    }
    if let Some((count, worst)) = pacing.sample(time.elapsed(), time.delta(), Some(rate)) {
        warn!(
            "Possible missed refresh: {count} late update interval(s), worst {:.2} ms; \
             {:.3} Hz display, {:.2} ms budget. Actual presentation timing unavailable.",
            worst.as_secs_f64() * 1000.0,
            rate as f64 / 1000.0,
            1_000_000.0 / rate as f64,
        );
    }
}

#[cfg(test)]
#[path = "tests/frame_pacing.rs"]
mod tests;
