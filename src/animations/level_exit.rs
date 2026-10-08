//! Level exit accelerates the glider down the tunnel, expands spinning claw
//! trails, and flashes a warp before advancing to the next level.

use crate::animations::glider_death as shot_death;
use crate::camera::CAMERA_STEP;
use crate::enemies::flippers::Flipper;
use crate::enemies::shots::EnemyShot;
use crate::enemies::spikers;
use crate::explosion;
use crate::game::Game;
use crate::game::Status;
use crate::glider::Glider;
use crate::level::Level;
use crate::level::LoadReason;
use crate::powerups;
use crate::stars;
use crate::weapons::Bullet;
use crate::weapons::Weapon;
use crate::web::LANE_UNIT;
use crate::web::WORLD_UNIT;
use crate::web::Web;
use bevy::prelude::*;

pub(crate) const ZOOM_ACCELERATION: i32 = 0x400;

pub(crate) const FLASH_TICKS: u8 = 16;

#[derive(Clone, Copy)]
pub(crate) struct Exit {
    pub(crate) speed: i32,
    pub(crate) extra_depth: i32,
    pub(crate) camera_z: f32,
    pub(crate) rear: i32,
    pub(crate) web_end: f32,
    pub(crate) warp_end: i32,
    pub(crate) distance: i32,
    pub(crate) depth: i32,
    pub(crate) ticks: u16,
    pub(crate) spin: u8,
    pub(crate) spacing: i32,
    pub(crate) wireframe: bool,
    pub(crate) web_visible: bool,
    pub(crate) flash: u8,
}

impl Exit {
    pub(crate) fn new(position: Vec3, web: &Web) -> Self {
        Self {
            speed: 0,
            extra_depth: 0,
            camera_z: web.camera_position(position).z,
            rear: web.rear(),
            web_end: 30.0 + f32::from(web.depth) + 70.0,
            warp_end: web.rear() + 1810 * LANE_UNIT,
            distance: 0,
            depth: ((0.12 - position.z) / WORLD_UNIT * LANE_UNIT as f32) as i32,
            ticks: 0,
            spin: 0,
            spacing: 0,
            wireframe: false,
            web_visible: true,
            flash: 0,
        }
    }

    pub(crate) fn advance(&mut self) -> bool {
        if self.flash > 0 {
            self.flash -= 1;
            return self.flash == 0;
        }
        self.ticks += 1;
        // zoom1 accelerates before running the claw; zoom2 runs it first.
        if self.web_visible {
            self.distance += self.speed;
            self.speed += ZOOM_ACCELERATION;
            self.depth += self.speed;
            if self.wireframe {
                self.spin = self.spin.wrapping_add(4);
                self.spacing += 0x4000;
            } else if self.depth >= self.rear {
                self.wireframe = true;
            }
            let source_camera =
                30.0 - self.camera_z / WORLD_UNIT + self.distance as f32 / LANE_UNIT as f32;
            if source_camera > self.web_end {
                self.web_visible = false;
            }
        } else {
            self.depth += self.speed + self.extra_depth;
            self.spin = self.spin.wrapping_add(4);
            // czoom2 and zoom2 each increase the trail spacing by a quarter unit.
            self.spacing += 0x8000;
            self.distance += self.speed;
            self.speed += ZOOM_ACCELERATION;
            self.extra_depth += 0x1000;
        }
        if self.depth >= self.warp_end {
            self.flash = FLASH_TICKS;
        }
        false
    }
}

#[derive(Component)]
pub(crate) struct WarpFlash;

pub(crate) fn setup(commands: &mut Commands) {
    commands.spawn((
        WarpFlash,
        BackgroundColor(Color::WHITE.with_alpha(0.0)),
        GlobalZIndex(100),
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            ..default()
        },
    ));
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn animate(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut player: Single<(&mut Glider, &mut Weapon, &mut Transform, &mut Visibility)>,
    mut cameras: Query<&mut Transform, (With<Camera3d>, Without<Glider>)>,
    actors: Query<
        (Entity, Option<&Bullet>),
        Or<(
            With<Bullet>,
            With<Flipper>,
            With<EnemyShot>,
            With<powerups::PowerUp>,
            With<explosion::Explosion>,
        )>,
    >,
    mut warp: Option<ResMut<stars::Warp>>,
    spikes: Query<&spikers::Spike>,
) {
    if game.status == Status::Bonus {
        return;
    }
    let Some(mut exit) = game.exit else {
        if let Some(warp) = &mut warp {
            **warp = stars::Warp::default();
        }
        return;
    };
    let (glider, weapon, transform, visibility) = &mut *player;
    let was_wireframe = exit.wireframe;
    if exit.advance() {
        game.exit = None;
        game.invulnerable = 0;
        game.load = Some(LoadReason::Advance);
        game.status = Status::Entering;
        **visibility = Visibility::Hidden;
        if let Some(warp) = &mut warp {
            **warp = stars::Warp::default();
        }
        return;
    }
    if exit.wireframe && !was_wireframe {
        for (entity, bullet) in &actors {
            if bullet.is_some() {
                commands.entity(entity).despawn();
            }
        }
    }
    transform.translation.z = 0.12 - exit.depth as f32 / LANE_UNIT as f32 * WORLD_UNIT;
    if exit.wireframe {
        glider.velocity = 0;
        **weapon = Weapon::default();
    }
    let target = level
        .web
        .camera_position(level.web.glider_transform(glider.progress).translation);
    for mut camera in &mut cameras {
        let delta = (target - camera.translation)
            .clamp(Vec3::splat(-CAMERA_STEP), Vec3::splat(CAMERA_STEP));
        camera.translation.x += delta.x;
        camera.translation.y += delta.y;
        camera.translation.z = exit.camera_z - exit.distance as f32 / LANE_UNIT as f32 * WORLD_UNIT;
    }
    if !exit.wireframe
        && exit.depth >= 0
        && spikes.iter().any(|spike| {
            spike.lane == level.web.selected_lane(glider.progress) as usize
                && exit.depth >= level.web.rear() - spike.length
        })
    {
        game.status = Status::Spiked;
        game.announce(crate::messages::Id::Spiked);
        game.exit = None;
        game.death = Some(shot_death::Death::retreat(
            exit.camera_z - exit.distance as f32 / LANE_UNIT as f32 * WORLD_UNIT,
        ));
        glider.velocity = 0;
        **weapon = Weapon::default();
        if let Some(warp) = &mut warp {
            **warp = stars::Warp::default();
        }
        return;
    }
    if let Some(warp) = &mut warp {
        warp.distance = exit.distance;
        warp.spacing = i32::from(exit.ticks) * 0x4000;
        warp.copies = 3;
    }
    game.exit = Some(exit);
}

pub(crate) fn draw(game: Res<Game>, mut flash: Single<&mut BackgroundColor, With<WarpFlash>>) {
    flash.0 = Color::WHITE.with_alpha(
        game.exit
            .map_or(0.0, |exit| f32::from(exit.flash) / f32::from(FLASH_TICKS)),
    );
}

#[cfg(test)]
#[path = "tests/level_exit.rs"]
mod tests;
