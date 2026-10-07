//! Flippers are red cross-shaped enemies that climb lanes and flip along the
//! rim toward the glider. Includes their spawn timing, poses, and movement.

use crate::enemies::Enemy;
use crate::enemies::beasts;
use crate::enemies::flipper_render::FlipperAssets;
use crate::enemies::fuseballs;
use crate::enemies::pulsars::Pulsar;
use crate::enemies::spikers;
use crate::enemies::tankers;
use crate::game::Game;
use crate::game::Status;
use crate::glider::Glider;
use crate::level::Level;
use crate::level::LevelActor;
use crate::web::FRONT_RADIUS;
use crate::web::LANE_UNIT;
use crate::web::WORLD_UNIT;
use crate::web::Web;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Motion {
    Approach,
    Climb,
    Flip {
        from: usize,
        direction: i32,
        angle: u16,
    },
    Pause(u8),
}

#[derive(Component, Debug)]
#[require(LevelActor, Enemy)]
pub(crate) struct Flipper {
    pub(crate) lane: usize,
    pub(crate) depth: i32,
    pub(crate) motion: Motion,
    pub(crate) pause: u8,
}

impl Flipper {
    pub(crate) fn preview_target(&self, web: &Web) -> Option<(usize, i32)> {
        self.hit_lane(web).map(|lane| (lane, self.depth))
    }

    pub(crate) fn hit_lane(&self, web: &Web) -> Option<usize> {
        match self.motion {
            Motion::Approach => None,
            Motion::Flip {
                from,
                direction,
                angle,
            } if web
                .flip_angle(from, self.lane, direction)
                .saturating_sub(angle)
                > 52 =>
            {
                None
            }
            _ => Some(self.lane),
        }
    }

    pub(crate) fn danger_lane(&self, web: &Web) -> Option<usize> {
        if self.depth > 2 * LANE_UNIT {
            return None;
        }
        match self.motion {
            Motion::Pause(_) => Some(self.lane),
            Motion::Flip {
                from,
                direction,
                angle,
            } if web
                .flip_angle(from, self.lane, direction)
                .saturating_sub(angle)
                > 52 =>
            {
                Some(from)
            }
            Motion::Flip { .. } => Some(self.lane),
            _ => None,
        }
    }
}

pub(crate) fn flipper_transform(flipper: &Flipper, web: &Web) -> Transform {
    let z = -(flipper.depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
    if let Motion::Flip {
        from,
        direction,
        angle,
    } = flipper.motion
    {
        let pivot = web.vertex(
            (from as i32 + i32::from(direction > 0)) as f32,
            FRONT_RADIUS,
            z,
        );
        let rotation = web.orientation(from)
            * Quat::from_rotation_z(direction as f32 * angle as f32 * TAU / 256.0);
        return Transform::from_translation(
            pivot + rotation * Vec3::X * (-direction as f32 * 9.0 * 0.075),
        )
        .with_rotation(rotation);
    }
    let mut transform = web.glider_transform(flipper.lane as i32 * LANE_UNIT);
    transform.translation.z = z;
    transform.rotation = web.orientation(flipper.lane);
    transform
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_enemies(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    assets: Res<FlipperAssets>,
    enemies: Query<&Flipper>,
    tankers: Query<&tankers::Tanker>,
    spikers: Query<&spikers::Spiker>,
    fuses: Query<&fuseballs::Fuseball>,
    pulsars: Query<&Pulsar>,
) {
    if game.status != Status::Playing || game.remaining == 0 {
        return;
    }
    if game.spawn_timer > 0 {
        game.spawn_timer -= 1;
        return;
    }
    if enemies.iter().count()
        + tankers.iter().count() * 3
        + spikers.iter().count()
        + fuses.iter().count()
        + pulsars.iter().count()
        >= level.flippers.max_active
    {
        return;
    }
    game.random = game.random.wrapping_mul(1664525).wrapping_add(1013904223);
    let flipper = Flipper {
        lane: (game.random >> 16) as usize % level.web.lanes(),
        depth: level.web.rear() + 300 * LANE_UNIT,
        motion: Motion::Approach,
        pause: level.flippers.pause_ticks,
    };
    commands.spawn((
        flipper_transform(&flipper, &level.web),
        flipper,
        RenderLayers::layer(1),
        Mesh3d(assets.point.clone()),
        MeshMaterial3d(assets.point_shades.first().cloned().unwrap_or_default()),
    ));
    game.remaining -= 1;
    game.spawn_timer = level.flippers.spawn_interval_ticks - 1;
}

pub(crate) fn start_flip(flipper: &mut Flipper, player_lane: usize, web: &Web) {
    if flipper.lane == player_lane {
        flipper.motion = Motion::Pause(flipper.pause);
        return;
    }
    let lanes = web.lanes();
    let clockwise = (player_lane + lanes - flipper.lane) % lanes;
    let direction =
        if (!web.closed && player_lane > flipper.lane) || (web.closed && clockwise < lanes / 2) {
            1
        } else {
            -1
        };
    let from = flipper.lane;
    flipper.lane = web.neighbor(from, direction).unwrap();
    flipper.motion = Motion::Flip {
        from,
        direction,
        angle: 0,
    };
}

#[allow(clippy::type_complexity)]
pub(crate) fn move_enemies(
    level: Res<Level>,
    game: Res<Game>,
    player: Single<&Glider>,
    assets: Res<FlipperAssets>,
    beast_assets: Res<beasts::BeastAssets>,
    mut enemies: Query<(
        &mut Flipper,
        &mut Transform,
        &mut Mesh3d,
        &mut MeshMaterial3d<StandardMaterial>,
        Option<&mut tankers::Released>,
        Option<&beasts::Beast>,
    )>,
) {
    if game.status != Status::Playing {
        return;
    }
    let player_lane = level.web.selected_lane(player.progress) as usize;
    for (mut flipper, mut transform, mut mesh, mut material, mut released, beast) in &mut enemies {
        let speed = if beast.is_some() {
            level
                .beasts
                .as_ref()
                .map_or(level.flippers.climb_speed, |g| g.climb_speed)
        } else {
            level.flippers.climb_speed
        };
        match flipper.motion {
            Motion::Approach => {
                flipper.depth -= 2 * LANE_UNIT;
                if flipper.depth <= level.web.rear() {
                    flipper.depth = level.web.rear() - speed;
                    flipper.motion = Motion::Climb;
                    mesh.0 = beast.map_or_else(
                        || assets.body.clone(),
                        |beast| beast_assets.frames[beast.stage].clone(),
                    );
                    material.0 = assets.shaded.clone();
                }
            }
            Motion::Climb => {
                flipper.depth = (flipper.depth - speed).max(0);
                if flipper.depth == 0 {
                    start_flip(&mut flipper, player_lane, &level.web);
                }
            }
            Motion::Flip {
                from,
                direction,
                angle,
            } => {
                let angle = angle
                    + level.flippers.flip_speed
                        * if released.as_ref().is_some_and(|r| r.fast) {
                            2
                        } else {
                            1
                        };
                flipper.motion = if angle >= level.web.flip_angle(from, flipper.lane, direction) {
                    Motion::Pause(if released.as_ref().is_some_and(|r| r.burst) {
                        0
                    } else {
                        flipper.pause
                    })
                } else {
                    Motion::Flip {
                        from,
                        direction,
                        angle,
                    }
                };
            }
            Motion::Pause(0) => {
                if let Some(released) = &mut released
                    && released.burst
                {
                    if released.remaining == 0 {
                        released.burst = false;
                        flipper.motion = Motion::Climb;
                    } else {
                        released.remaining -= 1;
                        let from = flipper.lane;
                        let direction = if level.web.neighbor(from, released.direction).is_none() {
                            -released.direction
                        } else {
                            released.direction
                        };
                        flipper.lane = level.web.neighbor(from, direction).unwrap();
                        flipper.motion = Motion::Flip {
                            from,
                            direction,
                            angle: 0,
                        };
                    }
                } else {
                    start_flip(&mut flipper, player_lane, &level.web);
                }
            }
            Motion::Pause(ticks) => flipper.motion = Motion::Pause(ticks - 1),
        }
        *transform = flipper_transform(&flipper, &level.web);
        if beast.is_some()
            && let Motion::Flip {
                from,
                direction,
                angle,
            } = flipper.motion
        {
            transform.rotation = level.web.orientation(from)
                * Quat::from_rotation_z(direction as f32 * angle as f32 * TAU / 128.);
        }
    }
}

#[cfg(test)]
#[path = "tests/flippers.rs"]
mod tests;
