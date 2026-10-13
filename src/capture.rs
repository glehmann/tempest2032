//! Screenshot and frame-sequence capture with scripted movement, firing,
//! and injected level-exit or enemy-shot-hit previews.

use crate::enemies::flipper_render::FlipperAssets;
use crate::enemies::flippers::Flipper;
use crate::enemies::fuseballs;
use crate::enemies::pulsars::Pulsar;
use crate::enemies::shots::EnemyShot;
use crate::enemies::spikers;
use crate::enemies::tankers;
use crate::game::Game;
use crate::glider::Glider;
use crate::level::Level;
use crate::level::LevelActor;
use crate::powerups;
use crate::powerups::Upgrades;
use crate::web::LANE_UNIT;
use crate::web::WORLD_UNIT;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::view::screenshot::Screenshot;
use bevy::render::view::screenshot::ScreenshotCaptured;
use bevy::render::view::screenshot::save_to_disk;
use std::path::PathBuf;

#[derive(Resource)]
pub(crate) struct CapturePath(pub(crate) PathBuf);

#[derive(Resource)]
pub(crate) struct MotionCapture {
    pub(crate) directory: PathBuf,
    pub(crate) tick: u32,
    pub(crate) next_capture: u32,
    pub(crate) frame: u32,
    pub(crate) finished: bool,
    pub(crate) end_tick: u32,
    pub(crate) preview: CapturePreview,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CapturePreview {
    Motion,
    Exit,
    ShotHit,
}

pub(crate) fn capture_frame(
    mut commands: Commands,
    path: Res<CapturePath>,
    mut frames: Local<u32>,
    glider: Single<&Visibility, With<Glider>>,
    game: Res<Game>,
    preview: Option<Res<crate::messages::Preview>>,
    front: Option<Res<crate::menu::FrontEnd>>,
) {
    if front.is_some_and(|front| front.screen != crate::menu::Screen::Gameplay) {
        // Menu previews have no visible glider or fixed gameplay clock.
    } else if let Some(preview) = preview {
        if preview.elapsed <= u32::from(preview.capture_tick.unwrap_or(60)) {
            return;
        }
    } else if game.status == crate::game::Status::Bonus {
        if game.bonus.as_ref().is_none_or(|session| session.tick < 60) {
            return;
        }
        if *frames < 60 {
            *frames = 59;
        }
    } else if *glider != Visibility::Visible {
        return;
    }
    *frames += 1;
    if *frames == 60 {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.0.clone()))
            .observe(
                |_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                    exit.write(AppExit::Success);
                },
            );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn preview_input(
    level: Res<Level>,
    mut capture: ResMut<MotionCapture>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    player: Single<&Glider>,
    pickups: Query<&powerups::PowerUp>,
    enemies: Query<&Flipper>,
    tankers: Query<&tankers::Tanker>,
    spikers: Query<&spikers::Spiker>,
    fuses: Query<&fuseballs::Fuseball>,
    pulsars: Query<&Pulsar>,
    mirrors: Query<&crate::enemies::mirrors::Mirror>,
    aliens: Query<&crate::enemies::aliens::AlienDroid>,
    upgrades: Res<Upgrades>,
    game: Res<Game>,
) {
    keys.release(KeyCode::KeyD);
    keys.release(KeyCode::KeyA);
    keys.release(KeyCode::Space);
    keys.release(KeyCode::KeyX);
    if game.status == crate::game::Status::Bonus {
        for key in [
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
            KeyCode::KeyW,
            KeyCode::KeyS,
        ] {
            keys.release(key);
        }
        if let Some(tunnel) = game
            .bonus
            .as_ref()
            .and_then(|session| session.tunnel.as_ref())
        {
            let current =
                tunnel.roll.position as f32 / 65536. + tunnel.roll.velocity as f32 / 65536. * 64.;
            let error = (f32::from(tunnel.target()) - current + 128.).rem_euclid(256.) - 128.;
            if error.abs() > 1. {
                keys.press(if error > 0. {
                    KeyCode::KeyD
                } else {
                    KeyCode::KeyA
                });
            }
        } else if let Some(session) = &game.bonus
            && let Some(gate) = session.gates.front()
        {
            let (x, y, _) = session.spec(gate.index);
            let position = session.position();
            let target = Vec2::new(f32::from(x), f32::from(y));
            let predicted = position
                + Vec2::new(
                    session.x.velocity as f32 / 65536. * 16.,
                    session.y.velocity as f32 / (8. * 65536.) * 4.,
                );
            if (target.x - predicted.x).abs() > 0.5 {
                keys.press(if target.x > predicted.x {
                    KeyCode::KeyD
                } else {
                    KeyCode::KeyA
                });
            }
            if (target.y - predicted.y).abs() > 0.5 {
                keys.press(if target.y > predicted.y {
                    KeyCode::KeyS
                } else {
                    KeyCode::KeyW
                });
            }
        }
        capture.tick += 1;
        return;
    }
    for key in [
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::KeyW,
        KeyCode::KeyS,
    ] {
        keys.release(key);
    }
    if upgrades.jump_enabled && pickups.is_empty() && capture.tick.is_multiple_of(120) {
        keys.press(KeyCode::KeyX);
    }
    if (30..capture.end_tick.saturating_sub(30)).contains(&capture.tick) {
        keys.press(KeyCode::Space);
    }
    let target = pickups
        .iter()
        .find_map(|pickup| pickup.preview_lane())
        .or_else(|| {
            enemies
                .iter()
                .filter_map(|enemy| enemy.preview_target(&level.web))
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
                .chain(
                    aliens
                        .iter()
                        .filter_map(crate::enemies::aliens::AlienDroid::target),
                )
                .min_by_key(|(_, depth)| *depth)
                .map(|(lane, _)| lane)
        });
    if let Some(target) = target {
        let lane = level.web.selected_lane(player.progress) as usize;
        let lanes = level.web.lanes();
        let clockwise = if level.web.closed {
            (target + lanes - lane) % lanes
        } else {
            target.abs_diff(lane)
        };
        if clockwise > 0 {
            keys.press(
                if (level.web.closed && clockwise < lanes / 2)
                    || (!level.web.closed && target > lane)
                {
                    KeyCode::KeyA
                } else {
                    KeyCode::KeyD
                },
            );
        }
    } else {
        match capture.tick % 360 {
            60..90 | 240..300 => keys.press(KeyCode::KeyD),
            150..180 => keys.press(KeyCode::KeyA),
            _ => {}
        }
    }
    capture.tick += 1;
}

pub(crate) fn capture_motion(mut commands: Commands, mut capture: ResMut<MotionCapture>) {
    if capture.finished || capture.tick < capture.next_capture {
        return;
    }
    let finished = capture.tick >= capture.end_tick;
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(
            capture.directory.join(format!("{:04}.png", capture.frame)),
        ))
        .observe(
            move |_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                if finished {
                    exit.write(AppExit::Success);
                }
            },
        );
    capture.frame += 1;
    capture.next_capture = capture.tick + 2;
    capture.finished = finished;
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn preview_transition(
    level: Res<Level>,
    mut commands: Commands,
    capture: Option<Res<MotionCapture>>,
    mut game: ResMut<Game>,
    actors: Query<Entity, With<LevelActor>>,
    player: Single<&Glider>,
    assets: Res<FlipperAssets>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
) {
    let Some(capture) = capture else {
        return;
    };
    if game.status == crate::game::Status::Bonus {
        return;
    }
    if capture.tick == 360 && capture.preview != CapturePreview::Motion {
        for entity in &actors {
            commands.entity(entity).despawn();
        }
        if capture.preview == CapturePreview::Exit {
            game.remaining = 0;
        } else {
            for key in [KeyCode::KeyZ, KeyCode::Space] {
                keys.release(key);
            }
            let shot = EnemyShot {
                lane: level.web.selected_lane(player.progress) as usize,
                depth: 2 * LANE_UNIT + LANE_UNIT / 2,
                angle: 0,
            };
            let mut transform = level.web.glider_transform(shot.lane as i32 * LANE_UNIT);
            transform.translation.z = -(shot.depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
            commands.spawn((
                shot,
                transform,
                RenderLayers::layer(1),
                Mesh3d(assets.shot.clone()),
                MeshMaterial3d(assets.shaded.clone()),
            ));
        }
    }
}
