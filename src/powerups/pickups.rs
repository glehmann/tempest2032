//! Power-up drops, collection, reward effects, and missed-pickup cleanup.
//! Pickups appear after eligible kills and travel toward the glider on a lane.

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
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn preview_lane(&self) -> Option<usize> {
        (!self.captured && self.depth >= 0).then_some(self.lane)
    }

    pub(crate) fn tick(&mut self, lane: usize, claw_depth: Option<i32>) -> bool {
        if self.rez >= 0 {
            self.rez -= 4;
        } else if self.captured {
            self.radius += 5;
        } else if self.lane == lane
            && claw_depth.is_some_and(|depth| (0..=32 * LANE_UNIT).contains(&(self.depth - depth)))
        {
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
    crate::sound::play(commands, crate::sound::Effect::Warp);
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
    tankers: Query<(Entity, &tankers::Tanker, &Transform)>,
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
    let claw_depth = match game.status {
        Status::Playing if upgrades.jump_height == 0 => Some(0),
        Status::Exiting => game
            .exit
            .filter(|exit| !exit.wireframe && exit.depth >= 0)
            .map(|exit| exit.depth),
        _ => None,
    };
    for (entity, mut pickup) in &mut pickups {
        if (pickup.captured || matches!(game.status, Status::Playing | Status::Exiting))
            && pickup.tick(lane, claw_depth)
        {
            let reward = upgrades.collect(&mut game);
            crate::sound::pitched(&mut commands, crate::sound::Effect::Warp, 428);
            game.announcements.extend(
                upgrades
                    .announcements
                    .drain(..)
                    .map(crate::messages::Event::from),
            );
            match reward {
                Reward::Droid => {
                    spawn_droid(&mut commands, &assets, &level.web);
                }
                Reward::Skip => {
                    game.outta_here = true;
                    for (entity, pose) in enemies
                        .iter()
                        .map(|(e, _, p)| (e, p))
                        .chain(tankers.iter().map(|(e, _, p)| (e, p)))
                        .chain(spikers.iter().map(|(e, _, p)| (e, p)))
                        .chain(fuses.iter().map(|(e, _, p)| (e, p)))
                        .chain(pulsars.iter().map(|(e, _, p)| (e, p)))
                        .chain(aliens.iter().map(|(e, _, p)| (e, p)))
                    {
                        commands.entity(entity).despawn();
                        explosion::spawn_explosion(&mut commands, &bursts, pose.translation);
                    }
                    for shot in &shots {
                        commands.entity(shot).despawn();
                    }
                    for entity in &mirrors {
                        commands.entity(entity).despawn();
                    }
                    game.remaining = 0;
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
