//! Bevy app startup and the ordered input, simulation, rendering, and game-flow
//! schedules. Keeps fixed-tick system ordering and run conditions together.

use crate::enemies::{pulsar_render, pulsars};
#[cfg(not(target_arch = "wasm32"))]
use bevy::diagnostic::SystemInformationDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PresentMode, WindowMode};

#[cfg(not(target_arch = "wasm32"))]
use crate::{
    capture::{
        CapturePath, CapturePreview, MotionCapture, capture_frame, capture_motion, preview_input,
        preview_transition,
    },
    frame_pacing::warn_late_frames,
};

use crate::{
    animations::{
        flipper_capture::animate_capture, glider_death as shot_death, level_entry as entry,
        level_exit as exit,
    },
    camera::{fit_camera, follow_glider},
    cli,
    collisions::resolve_hits,
    enemies::{
        aliens, beasts,
        flipper_render::{draw_approaching_flippers, update_enemy_colors},
        flippers::{move_enemies, spawn_enemies},
        fuseballs, mirrors,
        shots::{fire_enemy_shots, move_enemy_shots},
        spikers, tankers,
    },
    explosion,
    game::{Game, Status, check_game, controlling, load_level},
    glider::{animate_glider, move_glider},
    hud::{FpsMeter, spawn_fps_label, update_fps, update_hud},
    level::{Level, Levels},
    powerups,
    scene::{setup, sync_web, update_lane},
    stars, warp,
    weapons::{draw_particle_shots, fire_bullets, move_bullets},
};

#[derive(Resource)]
struct StartingLevel(usize);

#[derive(Resource)]
struct StartingWarp {
    kind: warp::ZoneKind,
    course: Option<u8>,
}

fn normal_scene(game: Res<Game>) -> bool {
    game.status != Status::Bonus
}

pub(crate) fn run(args: cli::Cli) {
    let mut app = App::new();
    let direct = args.direct_start();
    let store = crate::scores::Store::load();
    let difficulty = args.difficulty.unwrap_or(store.data.difficulty);
    if let Some(pattern) = args.starfield {
        app.insert_resource(stars::Animation::new(pattern));
    }
    crate::messages::add_systems(&mut app);
    if let Some(id) = args.message_preview {
        app.insert_resource(crate::messages::Preview {
            id,
            capture_tick: args.message_tick.or(args.screenshot.as_ref().map(|_| 60)),
            elapsed: 0,
        });
    }
    if args.fps {
        #[cfg(not(target_arch = "wasm32"))]
        app.add_plugins(SystemInformationDiagnosticsPlugin)
            .add_systems(Update, warn_late_frames);
        app.init_resource::<FpsMeter>()
            .add_systems(Startup, spawn_fps_label)
            .add_systems(Update, update_fps);
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(path) = args.screenshot {
        app.insert_resource(CapturePath(path))
            .add_systems(Update, capture_frame);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let recording = args
        .record_motion
        .map(|directory| {
            (
                directory,
                if args.warp_zone.is_some() { 30 } else { 6 },
                CapturePreview::Motion,
            )
        })
        .or_else(|| {
            args.record_exit
                .map(|directory| (directory, 14, CapturePreview::Exit))
        })
        .or_else(|| {
            args.record_shot_hit
                .map(|directory| (directory, 15, CapturePreview::ShotHit))
        });
    #[cfg(not(target_arch = "wasm32"))]
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
    let starting_level = args.level.unwrap_or(1) as usize;
    let mut levels = match Levels::load(&args.levels) {
        Ok(levels) => levels,
        Err(error) => {
            eprintln!("Could not load levels: {error}");
            std::process::exit(1);
        }
    };
    // Scale the parsed catalog, never the TOML, so the shipped source values
    // stay intact and every later level swap inherits the chosen difficulty.
    let eligible = !direct && levels.is_standard();
    let mut front = crate::menu::FrontEnd::new(
        if let Some(preview) = args.menu_preview {
            match preview {
                crate::menu::Preview::Welcome => crate::menu::Screen::Welcome,
                crate::menu::Preview::Configuration => crate::menu::Screen::Configuration,
                crate::menu::Preview::Initials => crate::menu::Screen::Initials,
            }
        } else if direct {
            crate::menu::Screen::Gameplay
        } else {
            crate::menu::Screen::Welcome
        },
        eligible,
        levels.0.clone(),
    );
    if matches!(args.menu_preview, Some(crate::menu::Preview::Initials)) {
        front.preview_initials();
    }
    app.insert_resource(front).insert_resource(store);
    let scale = difficulty.scale();
    for level in &mut levels.0 {
        level.scale_difficulty(scale);
    }
    app.insert_resource(difficulty);
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
    if let Some(kind) = args.warp_zone {
        app.insert_resource(StartingWarp {
            kind,
            course: args.warp_course,
        });
    }
    app.insert_resource(ClearColor(Color::BLACK))
        // All game materials are unlit; PBR still allocates fallback shadow maps.
        .insert_resource(bevy::light::PointLightShadowMap { size: 1 })
        .insert_resource(bevy::light::DirectionalLightShadowMap { size: 1 })
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                #[cfg(target_arch = "wasm32")]
                canvas: Some("#game".into()),
                #[cfg(target_arch = "wasm32")]
                fit_canvas_to_parent: true,
                title: format!("Tempest 2032 | Level {starting_level}"),
                resolution: args.message_size.map_or_else(
                    || (1100, 850).into(),
                    |size| {
                        bevy::window::WindowResolution::new(size.x, size.y)
                            .with_scale_factor_override(1.)
                    },
                ),
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
        .add_systems(
            Startup,
            (
                setup,
                warp::render::setup,
                crate::hud::setup,
                powerups::superzapper::setup,
            ),
        );
    add_systems(&mut app);
    app.insert_resource(bevy::audio::GlobalVolume::new(bevy::audio::Volume::Linear(
        args.volume,
    )));
    crate::sound::add_systems(&mut app);
    app.insert_resource(crate::sound::EffectsPitch(args.effects_pitch));
    crate::music::add_systems(&mut app);
    app.insert_resource(crate::music::MusicVolume(args.music_volume));
    crate::pause::add_systems(&mut app);
    crate::gamepad::add_systems(&mut app);
    crate::menu::add_systems(&mut app);
    #[cfg(target_arch = "wasm32")]
    crate::browser::add_systems(&mut app);
    if direct && args.warp_zone.is_none() {
        app.world_mut()
            .resource_mut::<Game>()
            .announce(crate::messages::Id::SuperzapperRecharge);
    }
    stars::add_systems(&mut app);
    app.run();
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
    let start_warp =
        app.world()
            .get_resource::<StartingWarp>()
            .map(|start| crate::level::LoadReason::Warp {
                kind: start.kind,
                course: start.course,
                destination: index,
            });
    app.init_resource::<Level>()
        .init_resource::<Levels>()
        .insert_resource(Game {
            status: Status::Entering,
            load: start_warp,
            pulsar_timer: initial
                .pulsars
                .as_ref()
                .map_or(0, |p| p.generator.first_spawn_ticks - 1),
            entry: Some(0),
            remaining: initial.flippers.count,
            spiral_flipper_timer: initial
                .spiral_flippers
                .as_ref()
                .map_or(0, |g| g.first_spawn_ticks - 1),
            alien_timer: initial
                .aliens
                .as_ref()
                .map_or(0, |g| g.first_spawn_ticks - 1),
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
            (powerups::queue_jump, powerups::superzapper::queue)
                .after(bevy::input::InputSystems)
                .run_if(normal_scene)
                .run_if(crate::pause::running)
                .run_if(crate::menu::playing),
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
                                | Status::Bonus
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
                    aliens::spawn,
                )
                    .chain(),
                (
                    powerups::superzapper::advance,
                    move_enemies,
                    mirrors::advance,
                    aliens::advance,
                    fuseballs::advance,
                    move_enemy_shots,
                    pulsars::check_sparks,
                    pulsars::advance,
                    resolve_hits,
                    pulsars::check_player,
                    aliens::check_player,
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
                    warp::advance,
                    check_game,
                    load_level,
                )
                    .chain(),
            )
                .chain()
                .run_if(crate::messages::running),
        )
        .add_systems(
            Update,
            (fit_camera.run_if(normal_scene), load_level, update_hud).chain(),
        )
        .add_systems(Update, sync_web.after(load_level))
        .add_systems(
            Update,
            warp::render::draw
                .after(load_level)
                .after(fit_camera)
                .after(sync_web),
        )
        .add_systems(Update, update_enemy_colors.after(load_level))
        .add_systems(Update, draw_approaching_flippers.after(fit_camera))
        .add_systems(Update, tankers::draw_points.after(fit_camera))
        .add_systems(Update, fuseballs::draw_points.after(fit_camera))
        .add_systems(Update, mirrors::draw_points.after(fit_camera))
        .add_systems(
            Update,
            (aliens::draw_points, aliens::draw_beams).after(fit_camera),
        )
        .add_systems(Update, draw_particle_shots.after(fit_camera))
        .add_systems(
            Update,
            powerups::superzapper::draw
                .after(fit_camera)
                .after(load_level),
        )
        .add_systems(
            Update,
            pulsar_render::draw.after(fit_camera).after(sync_web),
        )
        .add_systems(
            Update,
            (
                powerups::draw_powerups.after(fit_camera),
                entry::draw.after(load_level),
                exit::draw.after(load_level),
                shot_death::draw.after(load_level),
                spikers::draw.after(load_level),
            ),
        );
    #[cfg(not(target_arch = "wasm32"))]
    app.add_systems(
        FixedUpdate,
        preview_transition.after(preview_input).before(move_glider),
    );
}
