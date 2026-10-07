//! A Flipper capture drags the glider down the web, then retreats the camera
//! before retrying the level. Stores and advances that capture sequence.

use crate::animations::RETREAT_END;
use crate::enemies::flippers::Flipper;
use crate::game::Game;
use crate::game::Status;
use crate::glider::Glider;
use crate::level::Level;
use crate::web::LANE_UNIT;
use crate::web::WORLD_UNIT;
use bevy::prelude::*;

#[derive(Clone, Copy)]
pub(crate) struct Capture {
    pub(crate) flipper: Entity,
    pub(crate) depth: u16,
    pub(crate) camera_z: f32,
}

#[allow(clippy::type_complexity)]
pub(crate) fn animate_capture(
    level: Res<Level>,
    mut game: ResMut<Game>,
    mut player: Single<&mut Transform, With<Glider>>,
    mut enemies: Query<(&mut Flipper, &mut Transform), Without<Glider>>,
    mut cameras: Query<&mut Transform, (With<Camera3d>, Without<Glider>, Without<Flipper>)>,
) {
    let Some(mut capture) = game.capture else {
        return;
    };
    if game.status != Status::Captured {
        return;
    }
    if capture.depth < level.web.depth {
        let step = (level.web.depth - capture.depth).min(2);
        capture.depth += step;
        player.translation.z -= f32::from(step) * WORLD_UNIT;
        if let Ok((mut flipper, mut transform)) = enemies.get_mut(capture.flipper) {
            flipper.depth += i32::from(step) * LANE_UNIT;
            transform.translation.z -= f32::from(step) * WORLD_UNIT;
        }
    } else {
        capture.camera_z += 8.0 * WORLD_UNIT;
        for mut camera in &mut cameras {
            camera.translation.z = capture.camera_z;
        }
        // snatch_it_away stops at vp_z <= -400; webz-80 is our rim's zero.
        if capture.camera_z >= RETREAT_END {
            game.status = Status::Respawning;
            game.capture = None;
            for mut camera in &mut cameras {
                camera.translation = level
                    .web
                    .camera_position(level.web.glider_transform(level.web.start()).translation);
            }
            return;
        }
    }
    game.capture = Some(capture);
}
