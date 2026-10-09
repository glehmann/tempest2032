//! Rotating enemy shots fired by climbing Flippers and Spikers. Also advances
//! detached Beastie horns, which return toward the glider as reflected shots.

use crate::enemies::beasts;
use crate::enemies::flipper_render::FlipperAssets;
use crate::enemies::flippers::Flipper;
use crate::enemies::flippers::Motion;
use crate::enemies::mirrors;
use crate::enemies::spikers;
use crate::game::Game;
use crate::game::Status;
use crate::level::Level;
use crate::level::LevelActor;
use crate::mesh::colored_mesh;
use crate::web::LANE_UNIT;
use crate::web::WORLD_UNIT;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use std::f32::consts::TAU;

#[derive(Component)]
#[require(LevelActor)]
pub(crate) struct EnemyShot {
    pub(crate) lane: usize,
    pub(crate) depth: i32,
    pub(crate) angle: u8,
}

pub(crate) fn enemy_shot_mesh() -> Mesh {
    let vertices = [(1., 9.), (4., 9.), (5., 8.), (5., 10.)]
        .map(|(x, y)| Vec3::new((x - 9.) * 0.075, (9. - y) * 0.075, 0.0));
    let faces = [
        ([135, 104, 255], [(2, 192), (1, 255), (0, 64)]),
        ([224, 255, 71], [(3, 192), (1, 255), (0, 64)]),
    ];
    let mut triangles = Vec::new();
    for turn in 0..3 {
        let rotation = Quat::from_rotation_z(-(turn * 85) as f32 * TAU / 256.0);
        for (rgb, face) in faces {
            triangles.push(face.map(|(index, intensity)| {
                (
                    rotation * vertices[index],
                    Color::srgb_u8(
                        ((rgb[0] * intensity) >> 8) as u8,
                        ((rgb[1] * intensity) >> 8) as u8,
                        ((rgb[2] * intensity) >> 8) as u8,
                    ),
                )
            }));
        }
    }
    colored_mesh(triangles)
}

pub(crate) fn fire_enemy_shots(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    assets: Res<FlipperAssets>,
    enemies: Query<&Flipper>,
    shots: Query<&EnemyShot, (Without<beasts::Reflected>, Without<mirrors::ReturnedShot>)>,
    spikers: Query<&spikers::Spiker>,
) {
    if game.status != Status::Playing {
        return;
    }
    let sources = enemies
        .iter()
        .filter(|f| matches!(f.motion, Motion::Climb))
        .map(|f| (f.lane, f.depth))
        .chain(
            spikers
                .iter()
                .filter(|s| s.firing())
                .map(|s| (s.lane, s.depth)),
        );
    for (lane, depth) in sources {
        if game.fire_timer > 0 {
            game.fire_timer -= 1;
            continue;
        }
        game.fire_timer = 112;
        if depth <= 60 * LANE_UNIT || shots.iter().count() >= level.enemy_shots {
            continue;
        }
        let shot = EnemyShot {
            lane,
            depth,
            angle: 0,
        };
        crate::sound::play(&mut commands, crate::sound::Effect::Shot);
        commands.spawn((
            Transform::from_translation({
                let mut p = level
                    .web
                    .glider_transform(lane as i32 * LANE_UNIT)
                    .translation;
                p.z = -(depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
                p
            }),
            shot,
            RenderLayers::layer(1),
            Mesh3d(assets.shot.clone()),
            MeshMaterial3d(assets.shaded.clone()),
        ));
        break;
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn move_enemy_shots(
    mut commands: Commands,
    game: Res<Game>,
    mut shots: Query<(
        Entity,
        &mut EnemyShot,
        &mut Transform,
        Option<&beasts::Reflected>,
        Option<&mirrors::ReturnedShot>,
    )>,
) {
    if game.status != Status::Playing {
        return;
    }
    for (entity, mut shot, mut transform, reflection, returned) in &mut shots {
        shot.depth -= returned.map_or_else(|| reflection.map_or(0x18000, |r| r.speed), |r| r.speed);
        shot.angle = shot.angle.wrapping_add(4);
        transform.translation.z = -(shot.depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
        transform.rotation = Quat::from_rotation_z(-(shot.angle as f32) * TAU / 256.0);
        if shot.depth < 0 {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
#[path = "tests/shots.rs"]
mod tests;
