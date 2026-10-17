//! Player-shot hit resolution, nearest-hit selection, particle-laser piercing,
//! Beastie armor reflections, Tanker releases, spike trimming, and kill rewards.

use crate::enemies::pulsars::Pulsar;
use bevy::{camera::visibility::RenderLayers, prelude::*};
use std::collections::HashSet;

use crate::{
    enemies::{
        aliens, beasts,
        flipper_render::FlipperAssets,
        flippers::{Flipper, Motion},
        mirrors,
        shots::EnemyShot,
        spikers, tankers,
    },
    explosion,
    game::{Game, Status},
    level::Level,
    powerups,
    weapons::{Bullet, ParticleShot},
    web::{LANE_UNIT, WORLD_UNIT},
};

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn resolve_hits(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    bursts: Res<explosion::BurstAssets>,
    mut upgrades: ResMut<powerups::Upgrades>,
    power_assets: Res<powerups::PowerAssets>,
    bullets: Query<(Entity, &Bullet, &Transform, Option<&ParticleShot>)>,
    enemies: Query<
        (Entity, Option<&Flipper>, Option<&Pulsar>, &Transform),
        Or<(With<Flipper>, With<Pulsar>)>,
    >,
    enemy_shots: Query<(
        Entity,
        &EnemyShot,
        Option<&beasts::Reflected>,
        Option<&mirrors::ReturnedShot>,
    )>,
    tankers: Query<(Entity, &tankers::Tanker, &tankers::Contents, &Transform)>,
    shells: Res<tankers::TankerAssets>,
    (flipper_assets, fuse_assets, pulsar_assets, mirror_assets): (
        Res<FlipperAssets>,
        Res<crate::enemies::fuseballs::FuseAssets>,
        Res<crate::enemies::pulsar_render::PulsarAssets>,
        Res<mirrors::MirrorAssets>,
    ),
    (spikers, mut mirrors, mut aliens): (
        Query<(Entity, &spikers::Spiker, &Transform)>,
        Query<(Entity, &mut mirrors::Mirror, &Transform)>,
        Query<(Entity, &mut aliens::AlienDroid, &Transform)>,
    ),
    mut spikes: Query<(Entity, &mut spikers::Spike)>,
    mut beast_layers: Query<(&mut beasts::Beast, &Flipper, &mut Mesh3d)>,
    beast_assets: Res<beasts::BeastAssets>,
) {
    if !matches!(game.status, Status::Playing | Status::Exiting) {
        return;
    }
    let mut killed = HashSet::new();
    for (shot, bullet, transform, particle) in &bullets {
        let intersects = |lane, depth: i32| {
            let z = -(depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
            lane == bullet.lane && (z - transform.translation.z).abs() < WORLD_UNIT * 6.0
        };
        let mut hits = enemies
            .iter()
            .filter_map(|(entity, flipper, pulsar, transform)| {
                let (lane, depth, points) = if let Some(flipper) = flipper {
                    (flipper.hit_lane(&level.web)?, flipper.depth, 150)
                } else {
                    let pulsar = pulsar?;
                    let (lane, depth) = pulsar.target(&level.web)?;
                    (lane, depth, pulsar.points())
                };
                (!killed.contains(&entity) && intersects(lane, depth)).then_some((
                    entity,
                    depth,
                    points,
                    Some((lane, transform.translation)),
                ))
            })
            .chain(
                tankers
                    .iter()
                    .filter(|(entity, tanker, _, _)| {
                        !killed.contains(entity)
                            && !tanker.approach
                            && intersects(tanker.lane, tanker.depth)
                    })
                    .map(|(entity, tanker, _, _)| (entity, tanker.depth, 0, None)),
            )
            .chain(
                spikers
                    .iter()
                    .filter(|(entity, s, _)| {
                        !killed.contains(entity) && intersects(s.lane, s.depth)
                    })
                    .map(|(entity, s, pose)| {
                        (entity, s.depth, 150, Some((s.lane, pose.translation)))
                    }),
            )
            .chain(
                spikes
                    .iter()
                    .filter(|(entity, s)| {
                        !killed.contains(entity) && intersects(s.lane, level.web.rear() - s.length)
                    })
                    .map(|(entity, s)| (entity, level.web.rear() - s.length, 0, None)),
            )
            .chain(
                mirrors
                    .iter()
                    .filter(|(entity, mirror, _)| {
                        !killed.contains(entity)
                            && !mirror.approach
                            && intersects(mirror.lane, mirror.depth)
                    })
                    .map(|(entity, mirror, _)| (entity, mirror.depth, 0, None)),
            )
            .chain(
                enemy_shots
                    .iter()
                    .filter(|(entity, shot, _, _)| {
                        !killed.contains(entity) && intersects(shot.lane, shot.depth)
                    })
                    .map(|(entity, shot, _, _)| {
                        (
                            entity,
                            shot.depth,
                            0,
                            Some((shot.lane, transform.translation)),
                        )
                    }),
            )
            .chain(
                aliens
                    .iter()
                    .filter(|(entity, alien, _)| {
                        !killed.contains(entity)
                            && !alien.approach
                            && intersects(alien.lane, alien.depth)
                    })
                    .map(|(entity, alien, _)| (entity, alien.depth, 0, None)),
            )
            .collect::<Vec<_>>();
        hits.sort_unstable_by_key(|hit| hit.1);
        let nearest = hits.first().map(|hit| hit.0);
        for (entity, depth, points, centre) in hits {
            if particle.is_none() && Some(entity) != nearest {
                continue;
            }
            if let Ok((_, mut alien, pose)) = aliens.get_mut(entity) {
                let climbing = alien.climbing();
                if climbing || particle.is_none() {
                    commands.entity(shot).despawn();
                }
                let speed = level.aliens.as_ref().unwrap().climb_speed;
                if alien.hit(speed, level.web.rear()) {
                    commands.entity(entity).despawn();
                    killed.insert(entity);
                    let points = game.bonus_points();
                    game.add_score(points);
                    game.score_popup(points, pose.translation);
                    explosion::spawn_explosion(&mut commands, &bursts, pose.translation);
                } else {
                    commands.entity(entity).insert(alien.transform(&level.web));
                    explosion::spawn_explosion(&mut commands, &bursts, transform.translation);
                }
                if climbing {
                    break;
                }
                continue;
            }
            if let Ok((_, mut mirror, pose)) = mirrors.get_mut(entity) {
                let speed = level.mirrors.as_ref().unwrap().climb_speed;
                if mirror.hit(speed, level.web.rear()) {
                    commands.entity(entity).despawn();
                    killed.insert(entity);
                    let points = game.bonus_points();
                    game.add_score(points);
                    game.score_popup(points, pose.translation);
                    explosion::spawn_explosion(&mut commands, &bursts, pose.translation);
                    continue;
                }
                commands.entity(entity).insert(mirror.transform(&level.web));
                crate::sound::play(&mut commands, crate::sound::Effect::Shield);
                commands.entity(shot).despawn();
                commands.spawn((
                    EnemyShot {
                        lane: bullet.lane,
                        depth,
                        angle: 0,
                    },
                    mirrors::ReturnedShot {
                        speed: if particle.is_some() { 0x48000 } else { 0x38000 },
                    },
                    Transform::from_translation(transform.translation),
                    RenderLayers::layer(1),
                    Mesh3d(mirror_assets.reflection.clone()),
                    MeshMaterial3d(mirror_assets.material.clone()),
                ));
                break;
            }
            if particle.is_none() {
                commands.entity(shot).despawn();
            }
            if enemy_shots
                .get(entity)
                .is_ok_and(|(_, _, reflection, returned)| {
                    reflection.is_some() || returned.is_some()
                })
            {
                continue;
            }
            if let Ok((mut beast, flipper, mut mesh)) = beast_layers.get_mut(entity)
                && beast.stage > 0
                && matches!(flipper.motion, Motion::Climb)
            {
                let part = beast_assets.parts[beast.stage - 1].clone();
                crate::sound::play(&mut commands, crate::sound::Effect::Shield);
                beast.stage -= 1;
                mesh.0 = beast_assets.frames[beast.stage].clone();
                if particle.is_some() {
                    commands.entity(shot).despawn();
                }
                commands.spawn((
                    EnemyShot {
                        lane: bullet.lane,
                        depth,
                        angle: 0,
                    },
                    beasts::Reflected {
                        speed: if upgrades.laser { 0x24000 } else { 0x1c000 },
                    },
                    Transform::from_translation(transform.translation),
                    RenderLayers::layer(1),
                    Mesh3d(part),
                    MeshMaterial3d(flipper_assets.shaded.clone()),
                ));
                break;
            }
            if let Ok((_, mut spike)) = spikes.get_mut(entity) {
                crate::sound::pitched(
                    &mut commands,
                    crate::sound::Effect::Spike,
                    524 + ((spike.length / LANE_UNIT) & 511) as u16,
                );
                game.add_score(3);
                spike.length -= 3 * LANE_UNIT;
                if spike.length < 0 {
                    commands.entity(entity).despawn();
                    killed.insert(entity);
                    game.add_score(3);
                }
                continue;
            }
            commands.entity(entity).despawn();
            killed.insert(entity);
            game.add_score(points);
            if let Some((_, position)) = centre {
                game.score_popup(points, position);
            }
            if let Ok((_, tanker, contents, pose)) = tankers.get(entity) {
                tankers::open(
                    &mut commands,
                    &mut game,
                    &level,
                    &flipper_assets,
                    &shells,
                    &fuse_assets,
                    &pulsar_assets,
                    tanker,
                    *contents,
                );
                explosion::spawn_sphere(&mut commands, &bursts, pose.translation);
            }
            if let Some((lane, centre)) = centre {
                if upgrades.drop_due(level.powerups.kill_interval) {
                    if level.number <= 4 {
                        game.announce(crate::messages::Id::CollectPowerups);
                    }
                    powerups::spawn_drop(&mut commands, &power_assets, lane, depth, &level.web);
                } else {
                    explosion::spawn_explosion(&mut commands, &bursts, centre);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/collisions.rs"]
mod tests;
