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
    Bonus,
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
    pub(crate) super_flipper_timer: u16,
    pub(crate) spiral_flipper_timer: u16,
    pub(crate) mirror_timer: u16,
    pub(crate) extra_spiker_timer: u16,
    pub(crate) alien_timer: u16,
    pub(crate) bonus: Option<crate::warp::Session>,
    pub(crate) bonus_attempts: [u32; 3],
    pub(crate) announcements: Vec<crate::messages::Event>,
    pub(crate) message_epoch: u32,
    pub(crate) outta_here: bool,
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
            super_flipper_timer: 0,
            spiral_flipper_timer: 0,
            mirror_timer: 0,
            extra_spiker_timer: 0,
            alien_timer: 0,
            bonus: None,
            bonus_attempts: [0; 3],
            announcements: Vec::new(),
            message_epoch: 0,
            outta_here: false,
        }
    }
}

impl Game {
    pub(crate) fn announce(&mut self, id: crate::messages::Id) {
        self.announcements.push(id.into());
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
    game.spiral_flipper_timer = level
        .spiral_flippers
        .as_ref()
        .map_or(0, |g| g.first_spawn_ticks - 1);
    game.alien_timer = level.aliens.as_ref().map_or(0, |g| g.first_spawn_ticks - 1);
    game.extra_spiker_timer = level
        .extra_spikers
        .as_ref()
        .map_or(0, |g| g.first_spawn_ticks - 1);
    game.super_flipper_timer = level
        .super_flippers
        .as_ref()
        .map_or(0, |g| g.first_spawn_ticks - 1);
    game.mirror_timer = level
        .mirrors
        .as_ref()
        .map_or(0, |g| g.first_spawn_ticks - 1);
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
            game.announce(crate::messages::Id::CaughtYou);
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
            game.announce(crate::messages::Id::ShotYou);
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
        if (4..=16).contains(&level.number) && !game.outta_here {
            game.announce(crate::messages::Id::AvoidSpikes);
        }
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
    mut cameras: Query<
        (
            Entity,
            &mut Transform,
            Option<&mut Projection>,
            Option<&crate::warp::render::SavedCamera>,
        ),
        (With<Camera3d>, Without<Glider>),
    >,
    power_assets: Option<Res<powerups::PowerAssets>>,
    mut warp: Option<ResMut<stars::Warp>>,
    mut pulse: Option<ResMut<Pulse>>,
) {
    let Some(reason) = game.load.take() else {
        return;
    };
    let bonus = match reason {
        LoadReason::Warp {
            kind,
            course,
            destination,
        } => Some((kind, course, destination, false)),
        LoadReason::Advance if level.number < 99 && upgrades.warp_tokens() == 3 => {
            let destination = (game.level_index + 1) % levels.0.len();
            let kind = crate::warp::ZoneKind::for_web(levels.0[destination].number);
            Some((kind, None, destination, true))
        }
        _ => None,
    };
    if let Some((kind, course, destination, spend)) = bonus {
        if spend {
            upgrades.take_warp_tokens();
        }
        let index = kind.index();
        let attempt = game.bonus_attempts[index];
        game.bonus_attempts[index] = attempt.wrapping_add(1);
        game.bonus = Some(crate::warp::Session::new(
            kind,
            attempt,
            course,
            destination % levels.0.len(),
        ));
        info!(
            "Entering {:?} warp zone, returning to web {}",
            kind,
            levels.0[destination % levels.0.len()].number
        );
        game.status = Status::Bonus;
        game.entry = None;
        game.exit = None;
        game.capture = None;
        game.death = None;
        game.invulnerable = 0;
        upgrades.land();
        for entity in &actors {
            commands.entity(entity).despawn();
        }
        let (glider, weapon, _, visibility) = &mut *player;
        glider.velocity = 0;
        **weapon = Weapon::default();
        **visibility = Visibility::Hidden;
        for (entity, mut pose, projection, saved) in &mut cameras {
            if saved.is_none() {
                commands
                    .entity(entity)
                    .insert(crate::warp::render::SavedCamera {
                        pose: *pose,
                        projection: projection.as_deref().cloned(),
                    });
            }
            *pose = Transform::from_xyz(0., -crate::warp::SCALE, 0.);
        }
        if let Some(warp) = &mut warp {
            **warp = stars::Warp::default();
        }
        if let Some(pulse) = &mut pulse {
            **pulse = Pulse::default();
        }
        return;
    }
    match reason {
        LoadReason::Restart => {
            let epoch = game.message_epoch.wrapping_add(1);
            *game = Game::default();
            game.message_epoch = epoch;
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
        LoadReason::ReturnWarp { destination } => {
            game.level_index = destination % levels.0.len();
            game.invulnerable = 0;
        }
        LoadReason::Warp { .. } => unreachable!("warp entry handled above"),
    }
    for entity in &actors {
        commands.entity(entity).despawn();
    }
    *level = levels.0[game.level_index].clone();
    game.outta_here = false;
    upgrades.begin_level();
    game.capture = None;
    game.death = None;
    game.exit = None;
    game.entry = None;
    game.bonus = None;
    if game.lives > 0 {
        reset_wave(&mut game, &level);
        if !matches!(reason, LoadReason::Retry) {
            game.announce(crate::messages::Id::SuperzapperRecharge);
        }
    } else {
        game.status = Status::GameOver;
        game.announce(crate::messages::Id::GameOver);
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
    for (entity, mut camera, projection, saved) in &mut cameras {
        if let Some(saved) = saved {
            *camera = saved.pose;
            if let (Some(mut projection), Some(restore)) = (projection, saved.projection.as_ref()) {
                *projection = restore.clone();
            }
            commands
                .entity(entity)
                .remove::<crate::warp::render::SavedCamera>();
        }
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
