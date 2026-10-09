//! Buffered jump input and fixed-tick jump physics. The jump upgrade lifts
//! the glider off the rim to avoid enemy contact until it lands.

use crate::game::Game;
use crate::game::Status;
use crate::glider::Glider;
use crate::powerups::Upgrades;
use crate::web::LANE_UNIT;
use crate::web::WORLD_UNIT;
use bevy::prelude::*;

pub(crate) fn queue_jump(keys: Res<ButtonInput<KeyCode>>, mut upgrades: ResMut<Upgrades>) {
    upgrades.jump_queued |= upgrades.jump_enabled
        && keys.any_just_pressed([KeyCode::KeyJ, KeyCode::ShiftLeft, KeyCode::ShiftRight]);
}

pub(crate) fn update_jump(
    mut commands: Commands,
    game: Res<Game>,
    keys: Res<ButtonInput<KeyCode>>,
    mut upgrades: ResMut<Upgrades>,
    mut player: Single<&mut Transform, With<Glider>>,
) {
    if game.status != Status::Playing {
        return;
    }
    let requested = std::mem::take(&mut upgrades.jump_queued)
        || keys.any_pressed([KeyCode::KeyJ, KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    if upgrades.jump_height == 0 && upgrades.jump_enabled && requested {
        crate::sound::play(&mut commands, crate::sound::Effect::Jump);
        upgrades.jump_velocity = 2 * LANE_UNIT;
    }
    if upgrades.jump_velocity != 0 {
        upgrades.jump_height += upgrades.jump_velocity;
        upgrades.jump_velocity -= 0xc11;
        if upgrades.jump_height <= 0 {
            upgrades.jump_height = 0;
            upgrades.jump_velocity = 0;
        }
    }
    player.translation.z = 0.12 + upgrades.jump_height as f32 / LANE_UNIT as f32 * WORLD_UNIT;
}

#[cfg(test)]
#[path = "tests/jump.rs"]
mod tests;
