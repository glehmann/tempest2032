//! Bevy app startup and the ordered input, simulation, rendering, and game-flow
//! schedules. Keeps fixed-tick system ordering and run conditions together.

use crate::enemies::{pulsar_render, pulsars};
use bevy::diagnostic::SystemInformationDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PresentMode, WindowMode};

use crate::{
    animations::{
        flipper_capture::animate_capture, glider_death as shot_death, level_entry as entry,
        level_exit as exit,
    },
    camera::{fit_camera, follow_glider},
    capture::{
        CapturePath, CapturePreview, MotionCapture, capture_frame, capture_motion, preview_input,
        preview_transition,
    },
    cli,
    collisions::resolve_hits,
    enemies::{
        beasts,
        flipper_render::{draw_approaching_flippers, update_enemy_colors},
        flippers::{move_enemies, spawn_enemies},
        fuseballs, mirrors,
        shots::{fire_enemy_shots, move_enemy_shots},
        spikers, tankers,
    },
    explosion,
    frame_pacing::warn_late_frames,
    game::{Game, Status, check_game, controlling, load_level, restart_game},
    glider::{animate_glider, move_glider},
    hud::{FpsMeter, spawn_fps_label, update_fps, update_hud},
    level::{Level, Levels},
    powerups,
    scene::{setup, sync_web, update_lane},
    stars,
    weapons::{draw_particle_shots, fire_bullets, move_bullets},
};

#[derive(Resource)]
struct StartingLevel(usize);

pub(crate) fn run(args: cli::Cli) {
    let mut app = App::new();
    if args.fps {
        app.add_plugins(SystemInformationDiagnosticsPlugin)
            .init_resource::<FpsMeter>()
            .add_systems(Startup, spawn_fps_label)
            .add_systems(Update, (update_fps, warn_late_frames));
    }
    if let Some(path) = args.screenshot {
        app.insert_resource(CapturePath(path))
            .add_systems(Update, capture_frame);
    }
    let recording = args
        .record_motion
        .map(|directory| (directory, 6, CapturePreview::Motion))
        .or_else(|| {
            args.record_exit
                .map(|directory| (directory, 14, CapturePreview::Exit))
        })
        .or_else(|| {
            args.record_shot_hit
                .map(|directory| (directory, 15, CapturePreview::ShotHit))
        });
    if let Some((directory, default_seconds, preview)) = recording {
        std::fs::create_dir_all(&directory).expect("could not create motion capture directory");
        app.insert_resource(MotionCapture {
            directory,
            tick: 0,
            next_capture: 0,
            frame: 0,
            finished: false,
            end_tick: args.seconds.unwrap_or(default_seconds) * 60,
            preview,
        })
        .add_systems(FixedUpdate, preview_input.before(move_glider))
        .add_systems(Update, capture_motion);
    }
    let starting_level = args.level as usize;
    let levels = match Levels::load(&args.levels) {
        Ok(levels) => levels,
        Err(error) => {
            eprintln!("Could not load levels: {error}");
            std::process::exit(1);
        }
    };
    let initial = levels
        .0
        .get(starting_level - 1)
        .unwrap_or_else(|| {
            eprintln!(
                "Level {starting_level} does not exist; choose 1 through {}",
                levels.0.len()
            );
            std::process::exit(1);
        })
        .clone();
    app.insert_resource(initial)
        .insert_resource(levels)
        .insert_resource(StartingLevel(starting_level - 1));
    app.insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("T2K | Level {starting_level}"),
                resolution: (1100, 850).into(),
                present_mode: PresentMode::Fifo,
                mode: if args.fullscreen {
                    // A new window has no monitor yet, so this must not use Current.
                    WindowMode::BorderlessFullscreen(MonitorSelection::Primary)
                } else {
                    WindowMode::Windowed
                },
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup);
    add_systems(&mut app);
    stars::add_systems(&mut app);
    app.run();
}

fn quit_on_escape(keys: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

fn add_systems(app: &mut App) {
    let initial = app
        .world()
        .get_resource::<Level>()
        .cloned()
        .unwrap_or_default();
    let index = app
        .world()
        .get_resource::<StartingLevel>()
        .map_or(0, |start| start.0);
    app.init_resource::<Level>()
        .init_resource::<Levels>()
        .insert_resource(Game {
            status: Status::Entering,
            pulsar_timer: initial
                .pulsars
                .as_ref()
                .map_or(0, |p| p.generator.first_spawn_ticks - 1),
            entry: Some(0),
            remaining: initial.flippers.count,
            extra_spiker_timer: initial
                .extra_spikers
                .as_ref()
                .map_or(0, |g| g.first_spawn_ticks - 1),
            super_flipper_timer: initial
                .super_flippers
                .as_ref()
                .map_or(0, |g| g.first_spawn_ticks - 1),
            mirror_timer: initial
                .mirrors
                .as_ref()
                .map_or(0, |g| g.first_spawn_ticks - 1),
            spawn_timer: initial.flippers.first_spawn_ticks - 1,
            level_index: index,
            tanker_timer: initial
                .tankers
                .as_ref()
                .map_or(0, |g| g.first_spawn_ticks - 1),
            fuse_tanker_timer: initial
                .fuse_tankers
                .as_ref()
                .map_or(0, |g| g.first_spawn_ticks - 1),
            pulsar_tanker_timer: initial
                .pulsar_tankers
                .as_ref()
                .map_or(0, |p| p.generator.first_spawn_ticks - 1),
            spiker_timer: initial
                .spikers
                .as_ref()
                .map_or(0, |g| g.first_spawn_ticks - 1),
            beast_timer: initial
                .beasts
                .as_ref()
                .map_or(0, |g| g.first_spawn_ticks - 1),
            fuse_timer: initial
                .fuseballs
                .as_ref()
                .map_or(0, |g| g.first_spawn_ticks - 1),
            ..default()
        })
        .init_resource::<powerups::Upgrades>()
        .init_resource::<pulsars::Pulse>()
        .add_systems(
            PreUpdate,
            powerups::queue_jump.after(bevy::input::InputSystems),
        )
        .add_systems(
            FixedUpdate,
            preview_transition.after(preview_input).before(move_glider),
        )
        .add_systems(
            FixedUpdate,
            (
                (
                    move_glider.run_if(controlling),
                    powerups::update_jump,
                    animate_glider,
                    update_lane,
                    follow_glider.run_if(|game: Res<Game>| {
                        !matches!(
                            game.status,
                            Status::Entering
                                | Status::Captured
                                | Status::ShotHit
                                | Status::Spiked
                                | Status::Fried
                                | Status::Exiting
                        )
                    }),
                    move_bullets,
                    fire_bullets.run_if(controlling),
                    pulsars::pulse,
                    spawn_enemies,
                    tankers::spawn,
                    spikers::spawn,
                    beasts::spawn,
                    fuseballs::spawn,
                    pulsars::spawn,
                    mirrors::spawn,
                )
                    .chain(),
                (
                    move_enemies,
                    mirrors::advance,
                    fuseballs::advance,
                    move_enemy_shots,
                    pulsars::check_sparks,
                    pulsars::advance,
                    resolve_hits,
                    pulsars::check_player,
                    tankers::move_tankers,
                    spikers::advance,
                    fire_enemy_shots,
                    powerups::advance_powerups,
                    powerups::move_droid,
                )
                    .chain(),
                (
                    explosion::age_explosions,
                    entry::advance,
                    animate_capture,
                    shot_death::animate,
                    exit::animate,
                    check_game,
                    load_level,
                )
                    .chain(),
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                fit_camera,
                quit_on_escape,
                restart_game,
                load_level,
                update_hud,
            )
                .chain(),
        )
        .add_systems(Update, sync_web.after(load_level))
        .add_systems(Update, update_enemy_colors.after(load_level))
        .add_systems(Update, draw_approaching_flippers.after(fit_camera))
        .add_systems(Update, tankers::draw_points.after(fit_camera))
        .add_systems(Update, fuseballs::draw_points.after(fit_camera))
        .add_systems(Update, mirrors::draw_points.after(fit_camera))
        .add_systems(Update, draw_particle_shots.after(fit_camera))
        .add_systems(
            Update,
            pulsar_render::draw.after(fit_camera).after(sync_web),
        )
        .add_systems(
            Update,
            (
                powerups::draw_powerups.after(fit_camera),
                entry::draw.after(restart_game),
                exit::draw.after(restart_game),
                shot_death::draw.after(restart_game),
                spikers::draw.after(restart_game),
                powerups::update_notice,
            ),
        );
}
