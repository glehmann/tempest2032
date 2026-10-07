//! Screenshot and frame-sequence capture with scripted movement, firing,
//! and injected level-exit or enemy-shot-hit previews.

use crate::enemies::flipper_render::FlipperAssets;
use crate::enemies::flippers::Flipper;
use crate::enemies::fuseballs;
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
) {
    if *glider != Visibility::Visible {
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
    upgrades: Res<Upgrades>,
) {
    keys.release(KeyCode::KeyD);
    keys.release(KeyCode::KeyA);
    keys.release(KeyCode::Space);
    keys.release(KeyCode::KeyJ);
    if upgrades.jump_enabled && pickups.is_empty() && capture.tick.is_multiple_of(120) {
        keys.press(KeyCode::KeyJ);
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
    if capture.tick == 360 && capture.preview != CapturePreview::Motion {
        for entity in &actors {
            commands.entity(entity).despawn();
        }
        if capture.preview == CapturePreview::Exit {
            game.remaining = 0;
        } else {
            for key in [KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Space] {
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
