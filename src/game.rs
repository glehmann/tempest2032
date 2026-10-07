//! Game state, score and lives, level advancement, retries, and restarting.
//! Chooses death and transition animations and clears level-owned actors.

use crate::enemies::pulsars::Pulse;
use bevy::prelude::*;

use crate::{
    animations::{flipper_capture::Capture, glider_death as shot_death, level_exit as exit},
    enemies::{Enemy, flippers::Flipper, shots::EnemyShot, tankers},
    explosion,
    glider::Glider,
    level::{Level, LevelActor, Levels, LoadReason},
    powerups, stars,
    weapons::{Bullet, Weapon},
    web::{LANE_UNIT, WORLD_UNIT},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Entering,
    Playing,
    Captured,
    ShotHit,
    Spiked,
    Fried,
    Respawning,
    Exiting,
    GameOver,
}

#[derive(Resource)]
pub(crate) struct Game {
    pub(crate) score: u32,
    pub(crate) lives: u8,
    pub(crate) status: Status,
    pub(crate) remaining: u16,
    pub(crate) spawn_timer: u16,
    pub(crate) invulnerable: u16,
    pub(crate) random: u32,
    pub(crate) fire_timer: u8,
    pub(crate) capture: Option<Capture>,
    pub(crate) entry: Option<u16>,
    pub(crate) exit: Option<exit::Exit>,
    pub(crate) death: Option<shot_death::Death>,
    pub(crate) level_index: usize,
    pub(crate) load: Option<LoadReason>,
    pub(crate) tanker_timer: u16,
    pub(crate) fuse_tanker_timer: u16,
    pub(crate) pulsar_tanker_timer: u16,
    pub(crate) spiker_timer: u16,
    pub(crate) beast_timer: u16,
    pub(crate) fuse_timer: u16,
    pub(crate) pulsar_timer: u16,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            score: 0,
            lives: 3,
            status: Status::Playing,
            // _nw1's counter runs from 24 through zero before becoming negative.
            remaining: 25,
            spawn_timer: 60,
            invulnerable: 0,
            random: 0x2000,
            fire_timer: 112,
            capture: None,
            entry: None,
            exit: None,
            death: None,
            level_index: 0,
            load: None,
            tanker_timer: 0,
            fuse_tanker_timer: 0,
            pulsar_tanker_timer: 0,
            spiker_timer: 0,
            beast_timer: 0,
            fuse_timer: 0,
            pulsar_timer: 0,
        }
    }
}

pub(crate) fn playing(game: Res<Game>) -> bool {
    game.status == Status::Playing
}

pub(crate) fn controlling(game: Res<Game>) -> bool {
    game.exit
        .is_some_and(|exit| !exit.wireframe && exit.flash == 0)
        || playing(game)
}

pub(crate) fn reset_wave(game: &mut Game, level: &Level) {
    game.remaining = level.flippers.count;
    game.spawn_timer = level.flippers.first_spawn_ticks - 1;
    game.status = Status::Entering;
    game.pulsar_timer = level
        .pulsars
        .as_ref()
        .map_or(0, |p| p.generator.first_spawn_ticks - 1);
    game.entry = Some(0);
    game.fire_timer = 112;
    game.tanker_timer = level
        .tankers
        .as_ref()
        .map_or(0, |g| g.first_spawn_ticks - 1);
    game.fuse_tanker_timer = level
        .fuse_tankers
        .as_ref()
        .map_or(0, |g| g.first_spawn_ticks - 1);
    game.pulsar_tanker_timer = level
        .pulsar_tankers
        .as_ref()
        .map_or(0, |p| p.generator.first_spawn_ticks - 1);
    game.spiker_timer = level
        .spikers
        .as_ref()
        .map_or(0, |g| g.first_spawn_ticks - 1);
    game.beast_timer = level.beasts.as_ref().map_or(0, |g| g.first_spawn_ticks - 1);
    game.fuse_timer = level
        .fuseballs
        .as_ref()
        .map_or(0, |g| g.first_spawn_ticks - 1);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn check_game(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut upgrades: ResMut<powerups::Upgrades>,
    bursts: Res<explosion::BurstAssets>,
    mut player: Single<(&mut Glider, &mut Weapon, &mut Transform, &mut Visibility)>,
    enemies: Query<(Entity, &Flipper, Option<&tankers::Released>)>,
    living: Query<(), With<Enemy>>,
    bullets: Query<Entity, With<Bullet>>,
    enemy_shots: Query<(Entity, &EnemyShot)>,
    pickups: Query<(Entity, &powerups::PowerUp)>,
    droids: Query<Entity, With<powerups::Droid>>,
) {
    if !matches!(game.status, Status::Playing | Status::Respawning) {
        return;
    }
    if game.entry.is_some_and(|tick| tick <= 256) {
        return;
    }
    let (glider, weapon, transform, visibility) = &mut *player;
    let grabber = (!weapon.firing)
        .then(|| {
            enemies.iter().find(|(_, flipper, released)| {
                released.is_none_or(|released| !released.burst)
                    && flipper.danger_lane(&level.web)
                        == Some(level.web.selected_lane(glider.progress) as usize)
            })
        })
        .flatten()
        .map(|(entity, _, _)| entity);
    if game.status == Status::Playing && game.invulnerable > 0 {
        game.invulnerable -= 1;
        **visibility = if game.invulnerable > 0 && (game.invulnerable / 6).is_multiple_of(2) {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    } else if game.status == Status::Respawning
        || (upgrades.jump_height == 0
            && (grabber.is_some()
                || enemy_shots.iter().any(|(_, shot)| {
                    shot.lane == level.web.selected_lane(glider.progress) as usize
                        && shot.depth <= 2 * LANE_UNIT
                })))
    {
        if game.status == Status::Playing
            && let Some(flipper) = grabber
        {
            game.status = Status::Captured;
            game.capture = Some(Capture {
                flipper,
                depth: 0,
                camera_z: level.web.camera_position(transform.translation).z,
            });
            glider.velocity = 0;
            **weapon = Weapon::default();
            **visibility = Visibility::Visible;
            return;
        }
        if game.status == Status::Playing {
            game.status = Status::ShotHit;
            game.death = Some(shot_death::Death::new(transform.translation, &level.web));
            glider.velocity = 0;
            **weapon = Weapon::default();
            **visibility = Visibility::Hidden;
            upgrades.lose_life();
            for entity in &bullets {
                commands.entity(entity).despawn();
            }
            for entity in &droids {
                commands.entity(entity).despawn();
            }
            for (entity, shot) in &enemy_shots {
                if shot.lane == level.web.selected_lane(glider.progress) as usize
                    && shot.depth <= 2 * LANE_UNIT
                {
                    let mut centre = transform.translation;
                    centre.z = -(shot.depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
                    commands.entity(entity).despawn();
                    explosion::spawn_explosion(&mut commands, &bursts, centre);
                }
            }
            return;
        }
        game.lives -= 1;
        if game.lives == 0 {
            // The final reset also removes actors while leaving the web visible.
            game.load = Some(LoadReason::Retry);
            game.status = Status::GameOver;
        } else {
            game.load = Some(LoadReason::Retry);
            game.status = Status::Entering;
            game.invulnerable = 120;
        }
        return;
    }
    if game.remaining == 0
        && living.is_empty()
        && enemy_shots.is_empty()
        && pickups.iter().all(|(_, pickup)| pickup.captured)
    {
        game.status = Status::Exiting;
        game.entry = None;
        game.exit = Some(exit::Exit::new(transform.translation, &level.web));
        **weapon = Weapon::default();
        **visibility = Visibility::Visible;
        for entity in &bullets {
            commands.entity(entity).despawn();
        }
    }
}

pub(crate) fn restart_game(keys: Res<ButtonInput<KeyCode>>, mut game: ResMut<Game>) {
    if keys.just_pressed(KeyCode::KeyR) {
        game.load = Some(LoadReason::Restart);
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn load_level(
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut level: ResMut<Level>,
    levels: Res<Levels>,
    mut upgrades: ResMut<powerups::Upgrades>,
    mut player: Single<(&mut Glider, &mut Weapon, &mut Transform, &mut Visibility)>,
    actors: Query<Entity, With<LevelActor>>,
    mut cameras: Query<&mut Transform, (With<Camera3d>, Without<Glider>)>,
    power_assets: Option<Res<powerups::PowerAssets>>,
    mut warp: Option<ResMut<stars::Warp>>,
    mut pulse: Option<ResMut<Pulse>>,
) {
    let Some(reason) = game.load.take() else {
        return;
    };
    match reason {
        LoadReason::Restart => {
            *game = Game::default();
            *upgrades = powerups::Upgrades::default();
        }
        LoadReason::Retry => {
            upgrades.lose_life();
            game.invulnerable = if game.lives > 0 { 120 } else { 0 };
        }
        LoadReason::Advance => {
            game.invulnerable = 0;
            game.level_index = (game.level_index + 1) % levels.0.len();
            upgrades.land();
        }
    }
    for entity in &actors {
        commands.entity(entity).despawn();
    }
    *level = levels.0[game.level_index].clone();
    upgrades.begin_level();
    game.capture = None;
    game.death = None;
    game.exit = None;
    game.entry = None;
    if game.lives > 0 {
        reset_wave(&mut game, &level);
    } else {
        game.status = Status::GameOver;
    }
    if let Some(warp) = &mut warp {
        **warp = stars::Warp::default();
    }
    if let Some(pulse) = &mut pulse {
        **pulse = Pulse::default();
    }
    let (glider, weapon, transform, visibility) = &mut *player;
    glider.progress = level.web.start();
    glider.velocity = 0;
    **weapon = Weapon::default();
    **transform = level.web.glider_transform(level.web.start());
    **visibility = Visibility::Visible;
    for mut camera in &mut cameras {
        camera.translation = level.web.camera_position(transform.translation);
    }
    if upgrades.droid
        && let Some(assets) = power_assets
    {
        powerups::spawn_droid(&mut commands, &assets, &level.web);
    }
}

#[cfg(test)]
#[path = "tests/game.rs"]
mod tests;
