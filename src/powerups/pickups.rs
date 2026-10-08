//! Power-up drops, collection, reward effects, and missed-pickup cleanup.
//! Pickups appear after eligible kills and travel toward the glider on a lane.

use crate::enemies::flipper_render::FlipperAssets;
use crate::enemies::flippers::Flipper;
use crate::enemies::fuseballs;
use crate::enemies::pulsars::Pulsar;
use crate::enemies::shots::EnemyShot;
use crate::enemies::spikers;
use crate::enemies::tankers;
use crate::explosion;
use crate::game::Game;
use crate::game::Status;
use crate::glider::Glider;
use crate::level::Level;
use crate::level::LevelActor;
use crate::powerups::PowerAssets;
use crate::powerups::Reward;
use crate::powerups::Upgrades;
use crate::powerups::droid::spawn_droid;
use crate::web::LANE_UNIT;
use crate::web::WORLD_UNIT;
use crate::web::Web;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

#[derive(Component)]
#[require(LevelActor)]
pub(crate) struct PowerUp {
    pub(crate) lane: usize,
    pub(crate) depth: i32,
    pub(crate) rez: i16,
    pub(crate) radius: u16,
    pub(crate) captured: bool,
    pub(crate) mesh: Option<Handle<Mesh>>,
}

impl PowerUp {
    pub(crate) fn preview_lane(&self) -> Option<usize> {
        (!self.captured && self.depth >= 0).then_some(self.lane)
    }

    pub(crate) fn tick(&mut self, lane: usize, grounded: bool) -> bool {
        if self.rez >= 0 {
            self.rez -= 4;
        } else if self.captured {
            self.radius += 5;
        } else if grounded && self.lane == lane && (0..=32 * LANE_UNIT).contains(&self.depth) {
            self.captured = true;
            self.radius = 11;
            return true;
        } else {
            self.depth -= 0x18000;
        }
        false
    }
}

pub(crate) fn spawn_drop(
    commands: &mut Commands,
    assets: &PowerAssets,
    lane: usize,
    depth: i32,
    web: &Web,
) {
    let mut transform = web.glider_transform(lane as i32 * LANE_UNIT);
    transform.translation.z = -(depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
    commands.spawn((
        PowerUp {
            lane,
            depth,
            rez: 300,
            radius: 10,
            captured: false,
            mesh: None,
        },
        Mesh3d::default(),
        MeshMaterial3d(assets.material.clone()),
        transform,
        RenderLayers::layer(1),
        bevy::camera::visibility::NoFrustumCulling,
    ));
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn advance_powerups(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut upgrades: ResMut<Upgrades>,
    player: Single<&Glider>,
    assets: Res<PowerAssets>,
    bursts: Res<explosion::BurstAssets>,
    mut pickups: Query<(Entity, &mut PowerUp)>,
    enemies: Query<(Entity, &Flipper, &Transform)>,
    shots: Query<Entity, With<EnemyShot>>,
    tankers: Query<(Entity, &tankers::Tanker, &tankers::Contents, &Transform)>,
    shells: Res<tankers::TankerAssets>,
    (flippers, fuse_assets, pulsar_assets): (
        Res<FlipperAssets>,
        Res<fuseballs::FuseAssets>,
        Res<crate::enemies::pulsar_render::PulsarAssets>,
    ),
    (spikers, mirrors, aliens): (
        Query<(Entity, &spikers::Spiker, &Transform)>,
        Query<Entity, With<crate::enemies::mirrors::Mirror>>,
        Query<(Entity, &crate::enemies::aliens::AlienDroid, &Transform)>,
    ),
    fuses: Query<(Entity, &fuseballs::Fuseball, &Transform)>,
    pulsars: Query<(Entity, &Pulsar, &Transform)>,
) {
    upgrades.frame = upgrades.frame.wrapping_add(1);
    let lane = level.web.selected_lane(player.progress) as usize;
    for (entity, mut pickup) in &mut pickups {
        if (pickup.captured || game.status == Status::Playing)
            && pickup.tick(lane, upgrades.jump_height == 0)
        {
            match upgrades.collect(&mut game.score) {
                Reward::Droid => {
                    spawn_droid(&mut commands, &assets, &level.web);
                }
                reward @ (Reward::Zap | Reward::Skip) => {
                    if reward == Reward::Skip {
                        game.outta_here = true;
                    }
                    for (entity, alien, pose) in &aliens {
                        if reward == Reward::Skip || !alien.climbing() {
                            commands.entity(entity).despawn();
                            explosion::spawn_explosion(&mut commands, &bursts, pose.translation);
                            if reward == Reward::Zap {
                                game.random =
                                    game.random.wrapping_mul(1664525).wrapping_add(1013904223);
                                game.score += 250 * (1 + ((game.random >> 16) % 3));
                            }
                        }
                    }
                    for (entity, pulsar, pose) in &pulsars {
                        commands.entity(entity).despawn();
                        explosion::spawn_explosion(&mut commands, &bursts, pose.translation);
                        if reward == Reward::Zap {
                            game.score += pulsar.points();
                        }
                    }
                    for (entity, _, pose) in &fuses {
                        commands.entity(entity).despawn();
                        explosion::spawn_explosion(&mut commands, &bursts, pose.translation);
                        if reward == Reward::Zap {
                            game.score += 250;
                        }
                    }
                    for (entity, _, pose) in &spikers {
                        commands.entity(entity).despawn();
                        explosion::spawn_explosion(&mut commands, &bursts, pose.translation);
                        if reward == Reward::Zap {
                            game.score += 150;
                        }
                    }
                    for (entity, tanker, contents, transform) in &tankers {
                        if reward == Reward::Zap {
                            tankers::open(
                                &mut commands,
                                &mut game,
                                &level,
                                &flippers,
                                &shells,
                                &fuse_assets,
                                &pulsar_assets,
                                tanker,
                                *contents,
                            );
                        }
                        commands.entity(entity).despawn();
                        explosion::spawn_explosion(&mut commands, &bursts, transform.translation);
                    }
                    for (enemy, _, transform) in &enemies {
                        commands.entity(enemy).despawn();
                        explosion::spawn_explosion(&mut commands, &bursts, transform.translation);
                        if reward == Reward::Zap {
                            game.score += 150;
                        }
                    }
                    for shot in &shots {
                        commands.entity(shot).despawn();
                    }
                    if reward == Reward::Skip {
                        for entity in &mirrors {
                            commands.entity(entity).despawn();
                        }
                        game.remaining = 0;
                    }
                }
                _ => {}
            }
        }
        if pickup.radius >= 255 || pickup.depth < -80 * LANE_UNIT {
            if !pickup.captured {
                let mut centre = level
                    .web
                    .glider_transform(pickup.lane as i32 * LANE_UNIT)
                    .translation;
                centre.z = -(pickup.depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
                explosion::spawn_explosion(&mut commands, &bursts, centre);
            }
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
#[path = "tests/pickups.rs"]
mod tests;
