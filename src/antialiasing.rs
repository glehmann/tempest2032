//! Antialias geometry and the combined scene, retaining reductions only when they help.

use bevy::{anti_alias::fxaa::Fxaa, prelude::*, window::PrimaryWindow};
use clap::ValueEnum;
use std::collections::VecDeque;
use std::time::Duration;

/// Shaders, uploads, and level setup settle before the first reading.
const WARMUP: Duration = Duration::from_millis(1500);
const WINDOW: Duration = Duration::from_millis(500);
const SLOW_WINDOWS: u8 = 4;
const TRIAL: Duration = Duration::from_secs(2);
const COOLDOWN: Duration = Duration::from_secs(30);
const FRAME_BUDGET: Duration = Duration::from_nanos(16_666_667);
/// Allow 59.94 Hz displays and small scheduling variations, but react to a
/// sustained shortfall from the 60 Hz simulation. Include stalls in the average;
/// consecutive slow windows keep an isolated hitch from starting a trial.
const SLOW_FRAME: f64 = 1. / 57.;

/// A trial can restore the previous quality; confirmed reductions stay in place.
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

#[derive(Clone, Copy, Default)]
struct Timings {
    elapsed: Duration,
    frames: u32,
    excess: Duration,
}

impl Timings {
    fn sample(&mut self, delta: Duration) {
        self.elapsed += delta;
        self.frames += 1;
        self.excess += delta.saturating_sub(FRAME_BUDGET);
    }

    fn merge(&mut self, other: Self) {
        self.elapsed += other.elapsed;
        self.frames += other.frames;
        self.excess += other.excess;
    }

    fn mean(self) -> f64 {
        self.elapsed.as_secs_f64() / f64::from(self.frames.max(1))
    }

    fn excess(self) -> f64 {
        self.excess.as_secs_f64() / f64::from(self.frames.max(1))
    }

    fn improves(self, baseline: Self) -> bool {
        let faster = self.mean() <= baseline.mean() * 0.92
            && baseline.mean() - self.mean() >= 0.00075
            && self.excess() <= baseline.excess() + 0.0001;
        let smoother = self.excess() <= baseline.excess() * 0.75
            && baseline.excess() - self.excess() >= 0.0005
            && self.mean() <= baseline.mean() * 1.02;
        faster || smoother
    }
}

struct Trial {
    previous: Quality,
    baseline: Timings,
}

#[derive(PartialEq, Eq)]
struct Context {
    size: UVec2,
    game: Option<(usize, crate::game::Status)>,
    screen: Option<crate::menu::Screen>,
    paused: bool,
    warp: Option<std::mem::Discriminant<crate::warp::Phase>>,
}

#[derive(Resource)]
pub(crate) struct Antialiasing {
    pub(crate) quality: Quality,
    automatic: bool,
    warmup: Duration,
    window: Timings,
    baseline: Timings,
    slow_windows: u8,
    trial: Option<Trial>,
    cooldown: Duration,
    context: Option<Context>,
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
            automatic,
            warmup: WARMUP,
            window: Timings::default(),
            baseline: Timings::default(),
            slow_windows: 0,
            trial: None,
            cooldown: Duration::ZERO,
            context: None,
            prewarm,
        }
    }

    fn reset_measurement(&mut self) {
        if let Some(trial) = self.trial.take() {
            self.quality = trial.previous;
            self.cooldown = COOLDOWN;
            info!(
                "Antialiasing trial interrupted; restoring {}",
                self.quality.label()
            );
        }
        self.window = Timings::default();
        self.baseline = Timings::default();
        self.slow_windows = 0;
        self.warmup = WARMUP;
    }

    fn set_context(&mut self, context: Context) {
        if self.context.as_ref() != Some(&context) {
            self.reset_measurement();
            self.context = Some(context);
        }
    }

    fn sample(&mut self, delta: Duration, active: bool) {
        if !self.automatic {
            return;
        }
        if !active {
            self.reset_measurement();
            // The configurations are already built, so refocusing must not repeat them.
            self.prewarm.clear();
            return;
        }
        if !self.cooldown.is_zero() {
            self.cooldown = self.cooldown.saturating_sub(delta);
            return;
        }
        if self.quality == Quality::Off && self.trial.is_none() {
            return;
        }
        if !self.warmup.is_zero() {
            self.warmup = self.warmup.saturating_sub(delta);
            return;
        }
        self.window.sample(delta);
        if self.trial.is_some() {
            if self.window.elapsed < TRIAL {
                return;
            }
            let trial = self.trial.take().unwrap();
            let improved = self.window.improves(trial.baseline);
            if !improved {
                self.quality = trial.previous;
            }
            info!(
                "Antialiasing trial {}: {:.2} -> {:.2} ms/frame, {:.2} -> {:.2} ms/frame \
                 over the 60 Hz budget; {} {}",
                if improved { "helped" } else { "did not help" },
                trial.baseline.mean() * 1000.,
                self.window.mean() * 1000.,
                trial.baseline.excess() * 1000.,
                self.window.excess() * 1000.,
                if improved { "keeping" } else { "restoring" },
                self.quality.label(),
            );
            self.reset_measurement();
            self.cooldown = COOLDOWN;
            return;
        }
        if self.window.elapsed < WINDOW {
            return;
        }
        let window = std::mem::take(&mut self.window);
        if window.mean() > SLOW_FRAME {
            self.baseline.merge(window);
            self.slow_windows += 1;
            if self.slow_windows >= SLOW_WINDOWS {
                self.trial = Some(Trial {
                    previous: self.quality,
                    baseline: std::mem::take(&mut self.baseline),
                });
                let previous = self.quality;
                self.quality = self.quality.step_down();
                info!(
                    "Trying antialiasing {} instead of {} after sustained low frame rates",
                    self.quality.label(),
                    previous.label(),
                );
                self.slow_windows = 0;
                self.warmup = WARMUP;
            }
        } else {
            self.slow_windows = 0;
            self.baseline = Timings::default();
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
    virtual_time: Option<Res<Time<Virtual>>>,
    game: Option<Res<crate::game::Game>>,
    front: Option<Res<crate::menu::FrontEnd>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<Antialiasing>,
    mut cameras: Query<(&mut Msaa, Option<&mut Fxaa>), With<Camera>>,
) {
    state.set_context(Context {
        size: windows.iter().next().map_or(UVec2::ZERO, |w| {
            UVec2::new(w.physical_width(), w.physical_height())
        }),
        game: game.as_ref().map(|g| (g.level_index, g.status)),
        screen: front.as_ref().map(|f| f.screen),
        paused: virtual_time.is_some_and(|t| t.is_paused()),
        warp: game
            .as_ref()
            .and_then(|g| g.bonus.as_ref())
            .map(|s| std::mem::discriminant(&s.phase)),
    });
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

    /// Advances past the warmup, then runs the given frame intervals.
    fn measured(state: &mut Antialiasing, windows: &[Vec<Duration>]) {
        state.warmup = Duration::ZERO;
        for window in windows {
            for frame in window {
                state.sample(*frame, true);
            }
        }
    }

    fn begin_trial(state: &mut Antialiasing, seconds: f64) {
        state.warmup = Duration::ZERO;
        for _ in 0..1000 {
            frames(state, 1, seconds);
            if state.trial.is_some() {
                return;
            }
        }
        panic!("slow frames should start a trial");
    }

    fn finish_trial(state: &mut Antialiasing, seconds: f64) {
        // Model a settled pipeline; transition timings must not affect the result.
        state.warmup = Duration::ZERO;
        for _ in 0..1000 {
            frames(state, 1, seconds);
            if state.trial.is_none() {
                return;
            }
        }
        panic!("trial should finish");
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
    fn unchanged_or_worse_performance_restores_quality_and_waits_before_retrying() {
        for rate in [55., 56., 45.] {
            let mut state = Antialiasing::default();
            begin_trial(&mut state, 1. / 55.);
            assert_eq!(state.quality, Quality::Fxaa);
            finish_trial(&mut state, 1. / rate);
            assert_eq!(state.quality, Quality::Full);
            assert_eq!(state.cooldown, COOLDOWN);
            frames(&mut state, 600, 1. / 55.);
            assert_eq!(state.quality, Quality::Full);
            assert!(state.trial.is_none());
            frames(&mut state, 1200, 1. / 60.);
            begin_trial(&mut state, 1. / 55.);
            finish_trial(&mut state, 1. / 60.);
            assert_eq!(state.quality, Quality::Fxaa);
        }
    }

    #[test]
    fn only_helpful_reductions_are_kept_including_a_trial_with_no_filtering() {
        let mut state = Antialiasing::default();
        begin_trial(&mut state, 1. / 30.);
        finish_trial(&mut state, 1. / 60.);
        assert_eq!(state.quality, Quality::Fxaa);
        assert!(state.trial.is_none());
        frames(&mut state, 1800, 1. / 60.);
        begin_trial(&mut state, 1. / 30.);
        assert_eq!(state.quality, Quality::Off);
        assert!(state.trial.is_some());
        finish_trial(&mut state, 1. / 60.);
        assert_eq!(state.quality, Quality::Off);
        assert!(state.trial.is_none());
        frames(&mut state, 600, 1. / 120.);
        state.sample(Duration::from_secs(10), false);
        assert_eq!(state.quality, Quality::Off);
    }

    #[test]
    fn an_unhelpful_trial_with_no_filtering_restores_fxaa() {
        let mut state = Antialiasing::new(Quality::Fxaa, true);
        begin_trial(&mut state, 1. / 40.);
        assert_eq!(state.quality, Quality::Off);
        finish_trial(&mut state, 1. / 40.);
        assert_eq!(state.quality, Quality::Fxaa);
    }

    #[test]
    fn transition_frames_are_excluded_from_the_trial() {
        let mut state = Antialiasing::default();
        begin_trial(&mut state, 1. / 40.);
        frames(&mut state, 2, 0.5);
        assert_eq!(state.window.frames, 0);
        frames(&mut state, 1, 0.5);
        assert_eq!(state.window.frames, 0);
        finish_trial(&mut state, 1. / 60.);
        assert_eq!(state.quality, Quality::Fxaa);
    }

    #[test]
    fn smoother_pacing_can_help_even_when_the_mean_is_unchanged() {
        let timings = |intervals: &[f64]| {
            let mut timings = Timings::default();
            for interval in intervals {
                timings.sample(Duration::from_secs_f64(*interval));
            }
            timings
        };
        let choppy = timings(&[0.016, 0.016, 0.040, 0.006]);
        let steady = timings(&[0.0195; 4]);
        assert!((choppy.mean() - steady.mean()).abs() < 1e-9);
        assert!(steady.improves(choppy));
        assert!(!choppy.improves(steady));
        assert!(!timings(&[0.0205; 4]).improves(choppy));
        // A better average must not buy more time lost to stalls.
        assert!(!timings(&[0.010, 0.010, 0.010, 0.060]).improves(timings(&[0.025; 4])));
    }

    #[test]
    fn reaching_sixty_fps_keeps_the_trial_even_for_a_small_mean_improvement() {
        let mut state = Antialiasing::default();
        begin_trial(&mut state, 1. / 56.);
        finish_trial(&mut state, 1. / 60.);
        assert_eq!(state.quality, Quality::Fxaa);
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
        measured(&mut state, &vec![mixed; usize::from(SLOW_WINDOWS)]);
        assert_eq!(state.quality, Quality::Fxaa);
    }

    #[test]
    fn backgrounding_cancels_an_unfinished_trial() {
        let mut state = Antialiasing::new(Quality::Fxaa, true);
        frames(&mut state, 4, 0.5);
        assert_eq!(state.slow_windows, 1);
        assert_eq!(state.quality, Quality::Fxaa);
        state.sample(Duration::from_secs(10), false);
        assert_eq!(state.slow_windows, 0);
        assert_eq!(state.quality, Quality::Fxaa);
        begin_trial(&mut state, 1. / 30.);
        assert_eq!(state.quality, Quality::Off);
        state.sample(Duration::from_secs(10), false);
        assert_eq!(state.quality, Quality::Fxaa);
        assert!(state.trial.is_none());
        assert_eq!(state.cooldown, COOLDOWN);
    }

    #[test]
    fn scene_changes_cancel_trials_and_discard_old_baselines() {
        let context = |width, level, paused| Context {
            size: UVec2::new(width, 2160),
            game: Some((level, crate::game::Status::Playing)),
            screen: Some(crate::menu::Screen::Gameplay),
            paused,
            warp: None,
        };
        for changed in [
            context(1920, 0, false),
            context(3840, 1, false),
            context(3840, 0, true),
        ] {
            let mut state = Antialiasing::default();
            state.set_context(context(3840, 0, false));
            begin_trial(&mut state, 1. / 30.);
            state.set_context(changed);
            assert_eq!(state.quality, Quality::Full);
            assert!(state.trial.is_none());
            assert_eq!(state.baseline.frames, 0);
            assert_eq!(state.window.frames, 0);
            assert_eq!(state.cooldown, COOLDOWN);
        }
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
        let step = |app: &mut App, count, seconds| {
            for _ in 0..count {
                app.world_mut()
                    .resource_mut::<Time<Real>>()
                    .advance_by(Duration::from_secs_f64(seconds));
                app.update();
            }
        };
        step(&mut app, 110, 1. / 30.);
        assert_eq!(app.world().get::<Msaa>(cameras[0]), Some(&Msaa::Off));
        assert!(app.world().get::<Fxaa>(cameras[1]).unwrap().enabled);
        // No speedup: finish the trial at the same rate and restore every camera.
        step(&mut app, 120, 1. / 30.);
        for entity in cameras {
            assert_eq!(app.world().get::<Msaa>(entity), Some(&Msaa::Sample4));
        }
        assert!(app.world().get::<Fxaa>(cameras[1]).unwrap().enabled);
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
    fn sustained_fifty_two_to_fifty_five_fps_starts_a_trial() {
        for rate in [52., 53., 54., 55., 56.] {
            let mut state = Antialiasing::default();
            begin_trial(&mut state, 1. / rate);
            assert_eq!(state.quality, Quality::Fxaa, "{rate} FPS");
            assert!(state.trial.is_some());
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
        measured(&mut state, &vec![mixed; usize::from(SLOW_WINDOWS)]);
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
