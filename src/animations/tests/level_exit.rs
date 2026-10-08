//! Checks exit acceleration, spinning trails, warp flash, level progression, and restart cancellation.

use crate::animations::glider_death as shot_death;
use crate::animations::level_entry as entry;
use crate::animations::level_exit::Exit;
use crate::animations::level_exit::FLASH_TICKS;
use crate::animations::level_exit::WarpFlash;
use crate::animations::level_exit::{animate, draw, setup};
use crate::enemies::shots::EnemyShot;
use crate::explosion;
use crate::game::Game;
use crate::game::Status;
use crate::game::check_game;
use crate::game::controlling;
use crate::game::load_level;
use crate::game::restart_game;
use crate::glider::Glider;
use crate::glider::move_glider;
use crate::level::Level;
use crate::level::Levels;
use crate::powerups;
use crate::stars;
use crate::test_support::START_PROGRESS;
use crate::test_support::camera_position;
use crate::test_support::glider_transform;
use crate::weapons::Bullet;
use crate::weapons::ShotAssets;
use crate::weapons::Weapon;
use crate::weapons::fire_bullets;
use crate::web::LANE_UNIT;
use crate::web::WORLD_UNIT;
use bevy::prelude::*;

#[test]
fn zoom_accelerates_then_switches_to_spinning_trails_at_the_rear() {
    let mut exit = Exit::new(
        glider_transform(START_PROGRESS).translation,
        &Level::default().web,
    );
    for _ in 0..64 {
        assert!(!exit.advance());
    }
    assert_eq!(exit.distance, 31 * LANE_UNIT + LANE_UNIT / 2);
    assert_eq!(exit.depth, 32 * LANE_UNIT + LANE_UNIT / 2);
    assert!(!exit.wireframe);
    for _ in 64..142 {
        exit.advance();
    }
    assert!(!exit.wireframe);
    exit.advance();
    assert_eq!(exit.depth, 160 * LANE_UNIT + 7 * LANE_UNIT / 8);
    assert!(exit.wireframe);
    assert!(exit.web_visible);
    exit.advance();
    assert_eq!(exit.spin, 4);
    assert_eq!(exit.spacing, LANE_UNIT / 4);
    for _ in 144..183 {
        exit.advance();
    }
    assert!(exit.web_visible);
    exit.advance();
    assert!(!exit.web_visible);
    assert_eq!(exit.spacing, 10 * LANE_UNIT + LANE_UNIT / 4);
    exit.advance();
    assert_eq!(exit.spacing, 10 * LANE_UNIT + 3 * LANE_UNIT / 4);
    for _ in 185..359 {
        exit.advance();
    }
    assert_eq!(exit.flash, 0);
    exit.advance();
    assert_eq!(exit.flash, FLASH_TICKS);
    assert!(exit.depth >= (2000 - 30) * LANE_UNIT);
    for _ in 0..FLASH_TICKS - 1 {
        assert!(!exit.advance());
    }
    assert!(exit.advance());
}

fn app() -> App {
    let mut app = App::new();
    let mut meshes = Assets::<Mesh>::default();
    let mut materials = Assets::<StandardMaterial>::default();
    entry::setup(
        &mut app.world_mut().commands(),
        &mut meshes,
        &mut materials,
        &Level::default().web,
    );
    setup(&mut app.world_mut().commands());
    app.world_mut().flush();
    let mut keys = ButtonInput::default();
    keys.press(KeyCode::KeyA);
    keys.press(KeyCode::Enter);
    app.insert_resource(Game {
        remaining: 0,
        score: 9000,
        lives: 2,
        ..default()
    })
    .insert_resource(keys)
    .init_resource::<Level>()
    .init_resource::<Levels>()
    .insert_resource(meshes)
    .insert_resource(materials)
    .init_resource::<powerups::Upgrades>()
    .init_resource::<ShotAssets>()
    .init_resource::<explosion::BurstAssets>()
    .init_resource::<stars::Warp>()
    .add_systems(
        FixedUpdate,
        (
            move_glider.run_if(controlling),
            fire_bullets.run_if(controlling),
            entry::advance,
            shot_death::animate,
            animate,
            check_game,
            load_level,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (load_level.after(restart_game), entry::draw, draw).chain(),
    );
    app.world_mut().spawn((
        Glider {
            progress: START_PROGRESS,
            velocity: 0,
        },
        Weapon::default(),
        glider_transform(START_PROGRESS),
        Visibility::Visible,
    ));
    for _ in 0..2 {
        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_translation(camera_position(
                glider_transform(START_PROGRESS).translation,
            )),
        ));
    }
    app
}

fn ticks(app: &mut App, count: usize) {
    for _ in 0..count {
        app.world_mut().run_schedule(FixedUpdate);
        app.world_mut().run_schedule(Update);
    }
}

#[test]
fn exit_keeps_controls_until_the_rear_then_flashes_and_reenters_without_losing_rewards() {
    let mut app = app();
    app.world_mut().resource_mut::<powerups::Upgrades>().laser = true;
    app.world_mut()
        .resource_mut::<powerups::Upgrades>()
        .jump_enabled = true;
    ticks(&mut app, 1);
    assert!(app.world().resource::<Game>().status == Status::Exiting);
    ticks(&mut app, 64);
    assert_eq!(app.world().resource::<stars::Warp>().copies, 3);
    assert_eq!(
        app.world().resource::<stars::Warp>().spacing,
        16 * LANE_UNIT
    );
    assert!(app.world_mut().query::<&Bullet>().iter(app.world()).count() > 0);
    for camera in app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .iter(app.world())
    {
        assert!((camera.translation.z - (5.3 - 31.5 * WORLD_UNIT)).abs() < 1e-4);
    }
    ticks(&mut app, 79);
    let progress = app
        .world_mut()
        .query::<&Glider>()
        .single(app.world())
        .unwrap()
        .progress;
    assert_eq!(
        app.world_mut().query::<&Bullet>().iter(app.world()).count(),
        0
    );
    ticks(&mut app, 41);
    assert_eq!(
        app.world_mut()
            .query::<&Glider>()
            .single(app.world())
            .unwrap()
            .progress,
        progress
    );
    assert_eq!(
        *app.world_mut()
            .query_filtered::<&Visibility, With<entry::WebRoot>>()
            .single(app.world())
            .unwrap(),
        Visibility::Hidden
    );
    assert!(
        app.world_mut()
            .query_filtered::<&Visibility, With<entry::ClawTrail>>()
            .iter(app.world())
            .all(|visibility| *visibility == Visibility::Visible)
    );
    ticks(&mut app, 176);
    assert_eq!(
        app.world_mut()
            .query_filtered::<&BackgroundColor, With<WarpFlash>>()
            .single(app.world())
            .unwrap()
            .0
            .alpha(),
        1.0
    );
    ticks(&mut app, 16);
    let game = app.world().resource::<Game>();
    assert!(game.status == Status::Entering);
    assert_eq!(game.entry, Some(0));
    assert!(game.exit.is_none());
    assert_eq!((game.score, game.lives, game.remaining), (9000, 2, 25));
    assert!(app.world().resource::<powerups::Upgrades>().laser);
    assert!(app.world().resource::<powerups::Upgrades>().jump_enabled);
    assert_eq!(app.world().resource::<stars::Warp>().copies, 0);
    assert_eq!(
        app.world_mut()
            .query_filtered::<&BackgroundColor, With<WarpFlash>>()
            .single(app.world())
            .unwrap()
            .0
            .alpha(),
        0.0
    );
    for camera in app
        .world_mut()
        .query_filtered::<&Transform, With<Camera3d>>()
        .iter(app.world())
    {
        assert_eq!(
            camera.translation,
            camera_position(glider_transform(START_PROGRESS).translation)
        );
    }
}

#[test]
fn exits_advance_one_to_two_and_back_to_one_and_death_retries_two() {
    let mut app = app();
    let mut catalog =
        Levels::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("levels")).unwrap();
    catalog.0.truncate(2);
    app.insert_resource(catalog);
    ticks(&mut app, 377);
    assert_eq!(app.world().resource::<Level>().number, 2);
    assert_eq!(
        app.world_mut()
            .query::<&Glider>()
            .single(app.world())
            .unwrap()
            .progress,
        10 * LANE_UNIT
    );
    ticks(&mut app, 320);
    let progress = app
        .world_mut()
        .query::<&Glider>()
        .single(app.world())
        .unwrap()
        .progress;
    let lane = app.world().resource::<Level>().web.selected_lane(progress) as usize;
    app.world_mut().spawn((
        EnemyShot {
            lane,
            depth: LANE_UNIT,
            angle: 0,
        },
        Transform::default(),
    ));
    ticks(&mut app, 1);
    assert!(app.world().resource::<Game>().status == Status::ShotHit);
    ticks(&mut app, 163);
    assert_eq!(app.world().resource::<Level>().number, 2);
    assert_eq!(app.world().resource::<Game>().lives, 1);
    ticks(&mut app, 320);
    app.world_mut().resource_mut::<Game>().remaining = 0;
    ticks(&mut app, 377);
    assert_eq!(app.world().resource::<Level>().number, 1);
    assert_eq!(app.world().resource::<Game>().score, 9000);
    assert_eq!(app.world().resource::<Game>().lives, 1);
}

#[test]
fn all_configured_levels_advance_through_the_exit_and_wrap_to_one() {
    let mut app = app();
    app.insert_resource(
        Levels::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("levels")).unwrap(),
    );
    let count = app.world().resource::<Levels>().0.len();
    assert_eq!(count, 89);
    for next in (2..=count).chain(std::iter::once(1)) {
        app.world_mut().resource_mut::<Game>().remaining = 0;
        ticks(&mut app, 377);
        assert_eq!(app.world().resource::<Level>().number, next);
        assert_eq!(app.world().resource::<Game>().lives, 2);
        assert_eq!(app.world().resource::<Game>().score, 9000);
        assert!(app.world().resource::<Game>().status == Status::Entering);
        ticks(&mut app, 320);
        assert!(app.world().resource::<Game>().status == Status::Playing);
    }
}

#[test]
fn restart_cancels_the_flight_and_restores_normal_stars() {
    let mut app = app();
    app.add_systems(Update, restart_game.before(entry::draw).before(draw));
    ticks(&mut app, 200);
    assert!(app.world().resource::<Game>().exit.is_some());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyR);
    app.world_mut().run_schedule(Update);
    assert!(app.world().resource::<Game>().exit.is_none());
    assert!(app.world().resource::<Game>().status == Status::Entering);
    assert_eq!(app.world().resource::<Game>().score, 0);
    assert_eq!(app.world().resource::<stars::Warp>().distance, 0);
}
