//! The AI droid is a tumbling companion cube that follows nearby enemies
//! and fires additional shots from the web rim.

use crate::cry;
use crate::enemies::flipper_render::FlipperAssets;
use crate::enemies::flippers::Flipper;
use crate::enemies::flippers::Motion;
use crate::enemies::fuseballs;
use crate::enemies::pulsars::Pulsar;
use crate::enemies::spikers;
use crate::enemies::tankers;
use crate::explosion;
use crate::game::Game;
use crate::game::Status;
use crate::level::Level;
use crate::level::LevelActor;
use crate::mesh::colored_mesh;
use crate::powerups::PowerAssets;
use crate::powerups::Upgrades;
use crate::weapons::Bullet;
use crate::weapons::MAX_BULLETS;
use crate::weapons::ShotAssets;
use crate::weapons::ShotSlot;
use crate::web::LANE_UNIT;
use crate::web::WORLD_UNIT;
use crate::web::Web;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use std::f32::consts::TAU;

#[derive(Component)]
#[require(LevelActor)]
pub(crate) struct Droid {
    progress: i32,
    cooldown: u8,
}

pub(super) fn droid_mesh() -> Mesh {
    let corners: [Vec3; 8] = std::array::from_fn(|index| {
        Vec3::new(
            if index & 1 == 0 { -0.15 } else { 0.15 },
            if index & 2 == 0 { -0.15 } else { 0.15 },
            if index & 4 == 0 { -0.15 } else { 0.15 },
        )
    });
    let mut triangles = Vec::new();
    for index in 0..8 {
        for axis in [1, 2, 4] {
            if index & axis != 0 {
                continue;
            }
            let (a, b) = (corners[index], corners[index | axis]);
            let direction = (b - a).normalize();
            let u = direction
                .cross(if direction.z.abs() > 0.5 {
                    Vec3::Y
                } else {
                    Vec3::Z
                })
                .normalize()
                * 0.01;
            let v = direction.cross(u).normalize() * 0.01;
            let offsets = [-u - v, u - v, u + v, -u + v];
            for side in 0..4 {
                let next = (side + 1) % 4;
                let quad = [
                    a + offsets[side],
                    a + offsets[next],
                    b + offsets[next],
                    b + offsets[side],
                ]
                .map(|point| (point, Color::WHITE));
                triangles.extend([[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]]);
            }
        }
    }
    colored_mesh(triangles)
}

pub(crate) fn spawn_droid(commands: &mut Commands, assets: &PowerAssets, web: &Web) {
    let progress = (2 % web.lanes()) as i32 * LANE_UNIT;
    commands.spawn((
        Droid {
            progress,
            cooldown: 5,
        },
        Mesh3d(assets.droid.clone()),
        MeshMaterial3d(assets.droid_material.clone()),
        web.glider_transform(progress),
        RenderLayers::layer(1),
    ));
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn move_droid(
    level: Res<Level>,
    mut commands: Commands,
    game: Res<Game>,
    upgrades: Res<Upgrades>,
    assets: Res<FlipperAssets>,
    shots: Res<ShotAssets>,
    enemies: Query<&Flipper>,
    tankers: Query<&tankers::Tanker>,
    spikers: Query<&spikers::Spiker>,
    fuses: Query<&fuseballs::Fuseball>,
    pulsars: Query<&Pulsar>,
    mirrors: Query<&crate::enemies::mirrors::Mirror>,
    bullets: Query<Entity, With<ShotSlot>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    power_assets: Res<PowerAssets>,
    mut droids: Query<(&mut Droid, &mut Transform)>,
) {
    if game.status != Status::Playing {
        return;
    }
    if let Some(material) = materials.get_mut(&power_assets.droid_material) {
        material.base_color = cry::color(explosion::pulse_hue(upgrades.frame as u8), 255);
    }
    let target = enemies
        .iter()
        .filter(|flipper| !matches!(flipper.motion, Motion::Approach))
        .map(|flipper| (flipper.lane, flipper.depth))
        .chain(
            tankers
                .iter()
                .filter(|t| !t.approach)
                .map(|t| (t.lane, t.depth)),
        )
        .chain(spikers.iter().map(|s| (s.lane, s.depth)))
        .chain(fuses.iter().filter_map(|f| f.target()))
        .chain(pulsars.iter().filter_map(|p| p.target(&level.web)))
        .chain(
            mirrors
                .iter()
                .filter_map(crate::enemies::mirrors::Mirror::target),
        )
        .min_by_key(|(_, depth)| *depth)
        .map(|(lane, _)| lane);
    for (mut droid, mut transform) in &mut droids {
        if let Some(target) = target {
            let delta = level
                .web
                .lane_delta(droid.progress, target as i32 * LANE_UNIT);
            droid.progress = level
                .web
                .normalize(droid.progress + delta.clamp(-LANE_UNIT / 16, LANE_UNIT / 16));
        }
        let lane = droid.progress / LANE_UNIT;
        let fraction = (droid.progress % LANE_UNIT) as f32 / LANE_UNIT as f32;
        transform.translation = level
            .web
            .glider_transform(lane * LANE_UNIT)
            .translation
            .lerp(
                level
                    .web
                    .glider_transform((lane + 1) * LANE_UNIT)
                    .translation,
                fraction,
            )
            + Vec3::Z * (10.0 * WORLD_UNIT);
        let angle = upgrades.frame as f32 * TAU / 256.0;
        transform.rotation = Quat::from_euler(EulerRot::XYZ, angle, angle * 2.0, angle * 3.0);
        droid.cooldown = droid.cooldown.saturating_sub(1);
        if target.is_some() && droid.cooldown == 0 && bullets.iter().count() < MAX_BULLETS + 8 {
            let lane = level.web.selected_lane(droid.progress) as usize;
            let mut position = level
                .web
                .glider_transform(lane as i32 * LANE_UNIT)
                .translation;
            position.z = transform.translation.z;
            commands.spawn((
                Bullet { lane },
                RenderLayers::layer(1),
                Mesh3d(assets.shot.clone()),
                MeshMaterial3d(shots.material.clone()),
                Transform::from_translation(position),
            ));
            droid.cooldown = 6;
        }
    }
}

#[cfg(test)]
#[path = "tests/droid.rs"]
mod tests;
