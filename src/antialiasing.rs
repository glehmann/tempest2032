//! Antialias geometry and the combined scene, with a session-long slow-frame fallback.

use bevy::{anti_alias::fxaa::Fxaa, prelude::*, window::PrimaryWindow};
use clap::ValueEnum;
use std::collections::VecDeque;
use std::time::Duration;

/// Shaders, uploads, and level setup settle before the first reading.
const WARMUP: Duration = Duration::from_millis(1500);
const WINDOW: Duration = Duration::from_millis(500);
const SLOW_WINDOWS: u8 = 2;
/// Allow 59.94 Hz displays and small scheduling variations, but react to a
/// sustained shortfall from the 60 Hz simulation. Include stalls in the average;
/// two consecutive slow windows keep an isolated hitch from deciding the step.
const SLOW_FRAME: f64 = 1. / 57.;

/// Antialiasing steps down only, so a slow device settles instead of flapping.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, ValueEnum)]
pub(crate) enum Quality {
    #[default]
    Full,
    Fxaa,
    Off,
}

impl Quality {
    fn step_down(self) -> Self {
        match self {
            Self::Full => Self::Fxaa,
            Self::Fxaa | Self::Off => Self::Off,
        }
    }

    fn samples(self) -> Msaa {
        match self {
            Self::Full => Msaa::Sample4,
            Self::Fxaa | Self::Off => Msaa::Off,
        }
    }

    fn filtered(self) -> bool {
        self != Self::Off
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Full => "4x",
            Self::Fxaa => "fxaa",
            Self::Off => "off",
        }
    }
}

#[derive(Resource)]
pub(crate) struct Antialiasing {
    pub(crate) quality: Quality,
    start: Quality,
    automatic: bool,
    warmup: Duration,
    elapsed: Duration,
    frames: u32,
    slow_windows: u8,
    /// Remaining configurations to apply once, so a later step-down finds its
    /// render targets and pipelines already built instead of compiling them mid-game.
    prewarm: VecDeque<Quality>,
}

impl Default for Antialiasing {
    fn default() -> Self {
        Self::new(Quality::default(), true)
    }
}

impl Antialiasing {
    pub(crate) fn new(start: Quality, automatic: bool) -> Self {
        let mut prewarm = VecDeque::new();
        // Fixed-quality sessions never switch pipelines, so need no prewarming.
        if automatic && start != Quality::Off {
            prewarm.push_back(start);
            for quality in [Quality::Fxaa, Quality::Off] {
                if quality != start {
                    prewarm.push_back(quality);
                }
            }
        }
        Self {
            quality: start,
            start,
            automatic,
            warmup: WARMUP,
            elapsed: Duration::ZERO,
            frames: 0,
            slow_windows: 0,
            prewarm,
        }
    }

    fn sample(&mut self, delta: Duration, active: bool) {
        if !self.automatic || self.quality == Quality::Off {
            return;
        }
        if !active {
            *self = Self::new(self.start, self.automatic);
            // The configurations are already built, so refocusing must not repeat them.
            self.prewarm.clear();
            return;
        }
        if !self.warmup.is_zero() {
            self.warmup = self.warmup.saturating_sub(delta);
            return;
        }
        self.elapsed += delta;
        self.frames += 1;
        if self.elapsed < WINDOW {
            return;
        }
        let mean = self.elapsed / self.frames;
        self.elapsed = Duration::ZERO;
        self.frames = 0;
        if mean.as_secs_f64() > SLOW_FRAME {
            self.slow_windows += 1;
            if self.slow_windows >= SLOW_WINDOWS {
                self.quality = self.quality.step_down();
                info!(
                    "Antialiasing reduced to {} after sustained low frame rates",
                    self.quality.label()
                );
                self.slow_windows = 0;
                self.warmup = WARMUP;
            }
        } else {
            self.slow_windows = 0;
        }
    }
}

/// Starts from the host's own signals; the browser reports its renderer and pointer.
pub(crate) fn detected_start() -> Quality {
    #[cfg(target_arch = "wasm32")]
    {
        crate::browser::start_quality()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Quality::Full
    }
}

pub(crate) fn add_systems(app: &mut App, start: Quality, automatic: bool) {
    app.insert_resource(Antialiasing::new(start, automatic))
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
    // Sample count is part of wgpu's pipeline key, so an unprepared step-down stalls
    // on compilation. Pay that cost during the warmup instead.
    let applied = state.prewarm.pop_front().unwrap_or(state.quality);
    for (mut msaa, fxaa) in &mut cameras {
        let samples = applied.samples();
        if *msaa != samples {
            *msaa = samples;
        }
        if let Some(mut fxaa) = fxaa
            && fxaa.enabled != applied.filtered()
        {
            fxaa.enabled = applied.filtered();
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

    /// Advances past the warmup, then runs the given per-window frames.
    fn measured(state: &mut Antialiasing, windows: &[Vec<Duration>]) {
        state.warmup = Duration::ZERO;
        for window in windows {
            state.elapsed = Duration::ZERO;
            state.frames = 0;
            for frame in window {
                state.sample(*frame, true);
            }
        }
    }

    fn window(count: usize, seconds: f64) -> Vec<Duration> {
        vec![Duration::from_secs_f64(seconds); count]
    }

    #[test]
    fn startup_jitter_and_normal_refresh_rates_keep_antialiasing() {
        let mut state = Antialiasing::default();
        frames(&mut state, 3, 0.5);
        frames(&mut state, 600, 1. / 59.94);
        frames(&mut state, 240, 1. / 120.);
        frames(&mut state, 1, 0.2);
        frames(&mut state, 180, 1. / 60.);
        assert_eq!(state.quality, Quality::Full);
    }

    #[test]
    fn sustained_slow_rendering_steps_down_to_no_filtering_without_flapping() {
        let mut state = Antialiasing::default();
        frames(&mut state, 90, 1. / 60.);
        frames(&mut state, 80, 1. / 30.);
        assert_eq!(state.quality, Quality::Fxaa);
        frames(&mut state, 80, 1. / 30.);
        assert_eq!(state.quality, Quality::Off);
        frames(&mut state, 600, 1. / 120.);
        state.sample(Duration::from_secs(10), false);
        assert_eq!(state.quality, Quality::Off);
    }

    #[test]
    fn a_frame_rate_that_keeps_up_with_the_simulation_keeps_multisampling() {
        let mut state = Antialiasing::default();
        frames(&mut state, 90, 1. / 60.);
        // Leave room for small scheduling variations around the 60 Hz target.
        frames(&mut state, 300, 1. / 58.);
        assert_eq!(state.quality, Quality::Full);
        frames(&mut state, 300, 1. / 120.);
        assert_eq!(state.quality, Quality::Full);
    }

    #[test]
    fn one_hitch_inside_a_window_does_not_reduce_antialiasing() {
        let mut state = Antialiasing::default();
        state.warmup = Duration::ZERO;
        frames(&mut state, 12, 1. / 60.);
        frames(&mut state, 1, 0.2);
        frames(&mut state, 6, 1. / 60.);
        assert_eq!(state.quality, Quality::Full);
        frames(&mut state, 31, 1. / 60.);
        assert_eq!(state.quality, Quality::Full);
        assert_eq!(state.slow_windows, 0);
    }

    #[test]
    fn a_choppy_mix_of_refresh_lengths_counts_as_slow() {
        let mut state = Antialiasing::default();
        // Three fifths of the frames hit the refresh and the rest take twice as long.
        let mixed: Vec<Duration> = (0..25)
            .map(|index| Duration::from_secs_f64(if index % 5 < 3 { 1. / 60. } else { 1. / 30. }))
            .collect();
        measured(&mut state, &[mixed.clone(), mixed]);
        assert_eq!(state.quality, Quality::Fxaa);
    }

    #[test]
    fn backgrounding_resets_the_measurement_to_the_starting_state() {
        let mut state = Antialiasing::new(Quality::Fxaa, true);
        frames(&mut state, 4, 0.5);
        assert_eq!(state.slow_windows, 1);
        assert_eq!(state.quality, Quality::Fxaa);
        state.sample(Duration::from_secs(10), false);
        assert_eq!(state.slow_windows, 0);
        assert_eq!(state.quality, Quality::Fxaa);
        frames(&mut state, 80, 1. / 30.);
        assert_eq!(state.quality, Quality::Off);
        frames(&mut state, 180, 1. / 60.);
        assert_eq!(state.quality, Quality::Off);
    }

    #[test]
    fn a_starting_state_of_off_is_never_recovered() {
        let mut state = Antialiasing::new(Quality::Off, true);
        frames(&mut state, 600, 1. / 120.);
        state.sample(Duration::from_secs(10), false);
        assert_eq!(state.quality, Quality::Off);
    }

    #[test]
    fn fixed_quality_never_prewarms_or_falls_back_even_after_refocusing() {
        for quality in [Quality::Full, Quality::Fxaa, Quality::Off] {
            let mut app = App::new();
            app.init_resource::<Time<Real>>();
            add_systems(&mut app, quality, false);
            let window = app
                .world_mut()
                .spawn((Window::default(), PrimaryWindow))
                .id();
            let camera = app
                .world_mut()
                .spawn((Camera::default(), Msaa::Sample4, Fxaa::default()))
                .id();
            for frame in 0..180 {
                app.world_mut().get_mut::<Window>(window).unwrap().focused = frame != 90;
                app.world_mut()
                    .resource_mut::<Time<Real>>()
                    .advance_by(Duration::from_secs_f64(1. / 30.));
                app.update();
                let state = app.world().resource::<Antialiasing>();
                assert_eq!(state.quality, quality);
                assert!(state.prewarm.is_empty());
                assert_eq!(app.world().get::<Msaa>(camera), Some(&quality.samples()));
                assert_eq!(
                    app.world().get::<Fxaa>(camera).unwrap().enabled,
                    quality.filtered()
                );
            }
        }
    }

    #[test]
    fn the_fallback_switches_all_camera_passes_and_the_final_filter_together() {
        let mut app = App::new();
        app.init_resource::<Time<Real>>();
        add_systems(&mut app, Quality::Full, true);
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
        let step = |app: &mut App| {
            app.world_mut().resource_mut::<Antialiasing>().warmup = Duration::ZERO;
            for _ in 0..60 {
                app.world_mut()
                    .resource_mut::<Time<Real>>()
                    .advance_by(Duration::from_secs_f64(1. / 30.));
                app.update();
            }
        };
        step(&mut app);
        assert_eq!(app.world().get::<Msaa>(cameras[0]), Some(&Msaa::Off));
        assert!(app.world().get::<Fxaa>(cameras[1]).unwrap().enabled);
        step(&mut app);
        for entity in cameras {
            assert_eq!(app.world().get::<Msaa>(entity), Some(&Msaa::Off));
        }
        assert!(!app.world().get::<Fxaa>(cameras[1]).unwrap().enabled);
    }

    #[test]
    fn a_warm_window_spares_every_camera_pass() {
        let mut app = App::new();
        app.init_resource::<Time<Real>>();
        add_systems(&mut app, Quality::Full, true);
        app.world_mut().spawn((
            Window {
                focused: true,
                ..default()
            },
            PrimaryWindow,
        ));
        let camera = app
            .world_mut()
            .spawn((Camera::default(), Msaa::Sample4, Fxaa::default()))
            .id();
        for _ in 0..300 {
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .advance_by(Duration::from_secs_f64(1. / 60.));
            app.update();
        }
        assert_eq!(app.world().get::<Msaa>(camera), Some(&Msaa::Sample4));
        assert!(app.world().get::<Fxaa>(camera).unwrap().enabled);
    }

    #[test]
    fn the_alternate_configurations_are_built_during_the_warmup() {
        let mut app = App::new();
        app.init_resource::<Time<Real>>();
        add_systems(&mut app, Quality::Full, true);
        app.world_mut().spawn((
            Window {
                focused: true,
                ..default()
            },
            PrimaryWindow,
        ));
        let camera = app
            .world_mut()
            .spawn((Camera::default(), Msaa::Sample4, Fxaa::default()))
            .id();
        let advance = |app: &mut App| {
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .advance_by(Duration::from_secs_f64(1. / 60.));
            app.update();
        };
        assert!(!app.world().resource::<Antialiasing>().prewarm.is_empty());
        advance(&mut app);
        assert_eq!(app.world().get::<Msaa>(camera), Some(&Msaa::Sample4));
        advance(&mut app);
        assert_eq!(app.world().get::<Msaa>(camera), Some(&Msaa::Off));
        assert!(app.world().get::<Fxaa>(camera).unwrap().enabled);
        advance(&mut app);
        assert_eq!(app.world().get::<Msaa>(camera), Some(&Msaa::Off));
        assert!(!app.world().get::<Fxaa>(camera).unwrap().enabled);
        advance(&mut app);
        assert_eq!(app.world().get::<Msaa>(camera), Some(&Msaa::Sample4));
        assert!(app.world().get::<Fxaa>(camera).unwrap().enabled);
        assert_eq!(
            app.world().resource::<Antialiasing>().quality,
            Quality::Full
        );
    }

    #[test]
    fn refocusing_does_not_rebuild_the_configurations() {
        let mut app = App::new();
        app.init_resource::<Time<Real>>();
        add_systems(&mut app, Quality::Full, true);
        let window = app
            .world_mut()
            .spawn((
                Window {
                    focused: true,
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        let camera = app
            .world_mut()
            .spawn((Camera::default(), Msaa::Sample4))
            .id();
        for _ in 0..4 {
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .advance_by(Duration::from_secs_f64(1. / 60.));
            app.update();
        }
        app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
        app.update();
        assert!(app.world().resource::<Antialiasing>().prewarm.is_empty());
        assert_eq!(app.world().get::<Msaa>(camera), Some(&Msaa::Sample4));
    }

    #[test]
    fn sustained_fifty_two_to_fifty_five_fps_lowers_quality() {
        for rate in [52., 53., 54., 55., 56.] {
            let mut state = Antialiasing::default();
            measured(&mut state, &[window(30, 1. / rate), window(30, 1. / rate)]);
            assert_eq!(state.quality, Quality::Fxaa, "{rate} FPS");
        }
    }

    #[test]
    fn recurring_six_to_fifty_ms_intervals_are_not_discarded() {
        let mut state = Antialiasing::default();
        // Mostly on time, with a brief catch-up frame after each recurring stall.
        let mixed: Vec<Duration> = [1. / 60.; 7]
            .into_iter()
            .chain([0.050, 0.006])
            .cycle()
            .take(27)
            .map(Duration::from_secs_f64)
            .collect();
        measured(&mut state, &[mixed.clone(), mixed]);
        assert_eq!(state.quality, Quality::Fxaa);
    }

    #[test]
    fn occasional_fifty_ms_hitches_do_not_lower_quality() {
        let mut state = Antialiasing::default();
        state.warmup = Duration::ZERO;
        for _ in 0..4 {
            frames(&mut state, 1, 0.050);
            frames(&mut state, 120, 1. / 60.);
        }
        assert_eq!(state.quality, Quality::Full);
    }
}
