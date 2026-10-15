//! Antialias geometry and the combined scene, with a session-long slow-frame fallback.

use bevy::{anti_alias::fxaa::Fxaa, prelude::*, window::PrimaryWindow};
use std::time::Duration;

#[derive(Resource)]
struct Antialiasing {
    enabled: bool,
    warmup: Duration,
    elapsed: Duration,
    frames: u32,
    slow_windows: u8,
}

impl Default for Antialiasing {
    fn default() -> Self {
        Self {
            enabled: true,
            warmup: Duration::from_secs(5),
            elapsed: Duration::ZERO,
            frames: 0,
            slow_windows: 0,
        }
    }
}

impl Antialiasing {
    fn sample(&mut self, delta: Duration, active: bool) {
        if !self.enabled {
            return;
        }
        if !active {
            *self = Self::default();
            return;
        }
        if !self.warmup.is_zero() {
            self.warmup = self.warmup.saturating_sub(delta);
            return;
        }
        self.elapsed += delta;
        self.frames += 1;
        if self.elapsed < Duration::from_secs(1) {
            return;
        }
        // Allow 59.94 Hz displays and scheduling jitter without chasing individual stalls.
        if f64::from(self.frames) / self.elapsed.as_secs_f64() < 58. {
            self.slow_windows += 1;
        } else {
            self.slow_windows = 0;
        }
        self.elapsed = Duration::ZERO;
        self.frames = 0;
        if self.slow_windows >= 3 {
            self.enabled = false;
            info!("Antialiasing disabled after sustained low frame rates to prioritize 60 FPS");
        }
    }
}

pub(crate) fn add_systems(app: &mut App) {
    app.init_resource::<Antialiasing>()
        .add_systems(Update, update);
}

fn update(
    time: Res<Time<Real>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<Antialiasing>,
    mut cameras: Query<(&mut Msaa, Option<&mut Fxaa>), With<Camera>>,
) {
    let active = windows.iter().next().is_some_and(|window| {
        window.focused
            && window.visible
            && window.physical_width() > 0
            && window.physical_height() > 0
    });
    state.sample(time.delta(), active);
    let samples = if state.enabled {
        Msaa::Sample4
    } else {
        Msaa::Off
    };
    for (mut msaa, fxaa) in &mut cameras {
        if *msaa != samples {
            *msaa = samples;
        }
        if let Some(mut fxaa) = fxaa
            && fxaa.enabled != state.enabled
        {
            fxaa.enabled = state.enabled;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames(state: &mut Antialiasing, count: usize, seconds: f64) {
        for _ in 0..count {
            state.sample(Duration::from_secs_f64(seconds), true);
        }
    }

    #[test]
    fn startup_jitter_and_normal_refresh_rates_keep_antialiasing() {
        let mut state = Antialiasing::default();
        frames(&mut state, 10, 0.5);
        frames(&mut state, 600, 1. / 59.94);
        frames(&mut state, 240, 1. / 120.);
        frames(&mut state, 1, 0.2);
        frames(&mut state, 180, 1. / 60.);
        assert!(state.enabled);
    }

    #[test]
    fn sustained_slow_rendering_disables_antialiasing_without_flapping() {
        let mut state = Antialiasing::default();
        frames(&mut state, 300, 1. / 60.);
        frames(&mut state, 96, 1. / 30.);
        assert!(!state.enabled);
        frames(&mut state, 600, 1. / 120.);
        state.sample(Duration::from_secs(10), false);
        assert!(!state.enabled);
    }

    #[test]
    fn backgrounding_resets_the_measurement_instead_of_disabling_antialiasing() {
        let mut state = Antialiasing::default();
        frames(&mut state, 10, 0.5);
        frames(&mut state, 60, 1. / 30.);
        state.sample(Duration::from_secs(10), false);
        frames(&mut state, 10, 0.5);
        frames(&mut state, 60, 1. / 30.);
        assert!(state.enabled);
        frames(&mut state, 120, 1. / 60.);
        assert!(state.enabled);
    }

    #[test]
    fn fallback_switches_all_camera_passes_and_the_final_filter_together() {
        let mut app = App::new();
        app.init_resource::<Time<Real>>();
        add_systems(&mut app);
        app.world_mut().spawn((
            Window {
                focused: true,
                ..default()
            },
            PrimaryWindow,
        ));
        let cameras = [
            app.world_mut()
                .spawn((Camera::default(), Msaa::Sample4))
                .id(),
            app.world_mut()
                .spawn((Camera::default(), Msaa::Sample4, Fxaa::default()))
                .id(),
        ];
        app.world_mut().resource_mut::<Antialiasing>().warmup = Duration::ZERO;
        for _ in 0..96 {
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .advance_by(Duration::from_secs_f64(1. / 30.));
            app.update();
        }
        for entity in cameras {
            assert_eq!(app.world().get::<Msaa>(entity), Some(&Msaa::Off));
        }
        assert!(!app.world().get::<Fxaa>(cameras[1]).unwrap().enabled);
    }
}
