//! Persistent score, lives, frame-rate, processor, and memory indicators.

use crate::game::Game;
use bevy::diagnostic::{
    Diagnostic, DiagnosticPath, DiagnosticsStore, SystemInformationDiagnosticsPlugin,
};
use bevy::prelude::*;
use std::collections::VecDeque;
use std::time::Duration;

const FPS_REFRESH_INTERVAL: Duration = Duration::from_millis(500);
const FRAME_WINDOW: Duration = Duration::from_secs(2);

#[derive(Component)]
pub(crate) struct GameHud;

#[derive(Component)]
pub(crate) struct FpsLabel;

/// Counts frames between readings and retains frame times over a rolling window.
#[derive(Resource)]
pub(crate) struct FpsMeter {
    frames: u32,
    elapsed: Duration,
    refresh: Timer,
    frame_times: VecDeque<(Duration, f64)>,
}

impl Default for FpsMeter {
    fn default() -> Self {
        Self {
            frames: 0,
            elapsed: Duration::ZERO,
            refresh: Timer::new(FPS_REFRESH_INTERVAL, TimerMode::Repeating),
            frame_times: VecDeque::new(),
        }
    }
}

pub(crate) fn update_hud(game: Res<Game>, mut hud: Single<&mut Text, With<GameHud>>) {
    if !game.is_changed() {
        return;
    }
    let text = format!("{}\nLIVES {}", game.score, game.lives);
    if hud.0 != text {
        hud.0 = text;
    }
}

pub(crate) fn spawn_fps_label(mut commands: Commands) {
    commands.spawn((
        FpsLabel,
        Text::default(),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.4, 0.4, 0.5)),
        Node {
            position_type: PositionType::Absolute,
            top: px(48),
            right: px(26),
            ..default()
        },
    ));
}

pub(crate) fn update_fps(
    time: Res<Time<Real>>,
    diagnostics: Res<DiagnosticsStore>,
    mut meter: ResMut<FpsMeter>,
    mut label: Single<&mut Text, With<FpsLabel>>,
) {
    let delta = time.delta();
    if delta.is_zero() {
        return;
    }
    let now = time.elapsed();
    meter
        .frame_times
        .push_back((now, delta.as_secs_f64() * 1000.0));
    while meter
        .frame_times
        .front()
        .is_some_and(|(at, _)| now - *at > FRAME_WINDOW)
    {
        meter.frame_times.pop_front();
    }
    meter.frames += 1;
    meter.elapsed += delta;
    if !meter.refresh.tick(delta).just_finished() {
        return;
    }
    let fps = meter.frames as f64 / meter.elapsed.as_secs_f64();
    meter.frames = 0;
    meter.elapsed = Duration::ZERO;
    let (fastest, slowest) = meter
        .frame_times
        .iter()
        .fold((f64::MAX, 0.0_f64), |(fastest, slowest), (_, ms)| {
            (fastest.min(*ms), slowest.max(*ms))
        });
    let cpu = average(
        &diagnostics,
        &SystemInformationDiagnosticsPlugin::PROCESS_CPU_USAGE,
    )
    .map_or(String::new(), |usage| format!("  CPU {usage:.0}%"));
    let memory = average(
        &diagnostics,
        &SystemInformationDiagnosticsPlugin::PROCESS_MEM_USAGE,
    )
    .map_or(String::new(), |gib| {
        format!("  MEM {:.0} MiB", gib * 1024.0)
    });
    let text = format!("{fps:.0} FPS  {fastest:.1}-{slowest:.1} ms{cpu}{memory}");
    if label.0 != text {
        label.0 = text;
    }
}

fn average(diagnostics: &DiagnosticsStore, path: &DiagnosticPath) -> Option<f64> {
    diagnostics.get(path).and_then(Diagnostic::average)
}

#[cfg(test)]
#[path = "tests/hud.rs"]
mod tests;
