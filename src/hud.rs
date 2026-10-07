//! Score, lives, frame rate, and game-state messages such as capture, death,
//! and level clear.

use crate::game::Game;
use crate::game::Status;
use crate::level::Level;
use bevy::diagnostic::{Diagnostic, DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct GameHud;

#[derive(Component)]
pub(crate) struct FpsLabel;

pub(crate) fn update_hud(
    game: Res<Game>,
    level: Res<Level>,
    mut hud: Single<&mut Text, With<GameHud>>,
) {
    if !game.is_changed() {
        return;
    }
    let status = match game.status {
        Status::Entering | Status::Playing => "",
        Status::Captured | Status::Respawning => "\nCAUGHT YOU",
        Status::ShotHit => "\nSHOT YOU!",
        Status::Spiked => "\nSPIKED!",
        Status::Fried => "\nFRIED YOU!",
        Status::Exiting if level.spikers.is_some() => "\nLEVEL CLEAR\nAVOID THE SPIKES",
        Status::Exiting => "\nLEVEL CLEAR",
        Status::GameOver => "\nGAME OVER   R to restart",
    };
    let text = format!("{}\nLIVES {}{}", game.score, game.lives, status);
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
    diagnostics: Res<DiagnosticsStore>,
    mut label: Single<&mut Text, With<FpsLabel>>,
) {
    let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(Diagnostic::smoothed)
    else {
        return;
    };
    let text = format!("{fps:.0} FPS");
    if label.0 != text {
        label.0 = text;
    }
}

#[cfg(test)]
#[path = "tests/hud.rs"]
mod tests;
