//! Score, lives, and game-state messages such as capture, death, and level clear.

use crate::game::Game;
use crate::game::Status;
use crate::level::Level;
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct GameHud;

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
