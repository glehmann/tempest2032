//! Checks level loading, capture, life loss, grace periods, completion, and restarting.

use crate::camera::follow_glider;
use crate::enemies::flippers::Flipper;
use crate::enemies::flippers::Motion;
use crate::enemies::flippers::move_enemies;
use crate::game::Game;
use crate::game::Status;
use crate::game::check_game;
use crate::game::playing;
use crate::glider::Glider;
use crate::glider::move_glider;
use crate::level::Level;
use crate::level::Levels;
use crate::level::LoadReason;
use crate::powerups;
use crate::test_support::REAR;
use crate::test_support::START_PROGRESS;
use crate::test_support::app;
use crate::test_support::camera_position;
use crate::test_support::enemy;
use crate::test_support::finish_capture;
use crate::test_support::glider_transform;
use crate::test_support::selected_lane;
use crate::test_support::ticks;
use crate::weapons::Bullet;
use crate::weapons::ShotAssets;
use crate::weapons::fire_bullets;
use crate::web::LANE_UNIT;
use crate::web::WORLD_UNIT;
use bevy::prelude::*;

#[test]
fn score_awards_every_twenty_thousand_boundary_including_multiple_lives_at_once() {
    let mut game = Game::default();
    game.add_score(19_999);
    assert_eq!((game.score, game.lives), (19_999, 4));
    assert!(game.announcements.is_empty());
    game.add_score(1);
    assert_eq!((game.score, game.lives), (20_000, 5));
    game.add_score(79_999);
    assert_eq!((game.score, game.lives), (99_999, 8));
    game.add_score(1);
    assert_eq!((game.score, game.lives), (100_000, 9));
    game.add_score(0);
    assert_eq!(game.announcements.len(), 5);
    assert!(
        game.announcements
            .iter()
            .all(|event| event.id == crate::messages::Id::OneUp)
    );
    game.lives = 255;
    game.add_score(20_000);
    assert_eq!(
        game.lives, 256,
        "the eight HUD icons do not cap earned lives"
    );
}

#[test]
fn level_loading_distinguishes_advancing_retrying_and_restarting() {
    let mut app = app();
    let mut second = Level::default();
    second.number = 2;
    second.web.start_lane = 3;
    second.web.depth = 180;
    second.flippers.count = 10;
    second.flippers.first_spawn_ticks = 11;
    app.world_mut().resource_mut::<Levels>().0.push(second);
    app.world_mut().resource_mut::<Game>().score = 9000;
    app.world_mut().resource_mut::<Game>().lives = 2;
    app.world_mut().resource_mut::<powerups::Upgrades>().laser = true;
    app.world_mut().resource_mut::<powerups::Upgrades>().droid = true;
    let old = enemy(&mut app, 7, 0, Motion::Pause(0));
    app.world_mut().resource_mut::<Game>().load = Some(LoadReason::Advance);
    ticks(&mut app, 1);
    assert!(app.world().get_entity(old).is_err());
    assert_eq!(app.world().resource::<Level>().number, 2);
    assert_eq!(app.world().resource::<Game>().announcements.len(), 1);
    assert_eq!(
        app.world().resource::<Game>().announcements[0].id,
        crate::messages::Id::SuperzapperRecharge
    );
    assert_eq!(
        (
            app.world().resource::<Game>().score,
            app.world().resource::<Game>().lives,
            app.world().resource::<Game>().remaining
        ),
        (9000, 2, 10)
    );
    assert_eq!(app.world().resource::<Game>().spawn_timer, 10);
    assert!(!app.world().resource::<powerups::Upgrades>().laser);
    assert_eq!(
        app.world_mut()
            .query::<&powerups::Droid>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(
        app.world_mut()
            .query::<&Glider>()
            .single(app.world())
            .unwrap()
            .progress,
        3 * LANE_UNIT
    );
    app.world_mut().resource_mut::<Game>().announcements.clear();
    app.world_mut().resource_mut::<Game>().load = Some(LoadReason::Retry);
    ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Level>().number, 2);
    assert_eq!(app.world().resource::<Game>().announcements.len(), 1);
    assert_eq!(
        app.world().resource::<Game>().announcements[0].id,
        crate::messages::Id::SuperzapperRecharge
    );
    assert!(!app.world().resource::<powerups::Upgrades>().laser);
    assert_eq!(
        app.world_mut()
            .query::<&powerups::Droid>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(app.world().resource::<Game>().score, 9000);
    app.world_mut().resource_mut::<Game>().load = Some(LoadReason::Restart);
    ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Level>().number, 1);
    assert!(app.world().resource::<Game>().announcements.is_empty());
    assert_eq!(
        (
            app.world().resource::<Game>().score,
            app.world().resource::<Game>().lives
        ),
        (0, 4)
    );
    app.world_mut().resource_mut::<Game>().load = Some(LoadReason::Retry);
    ticks(&mut app, 1);
    assert!(app.world().resource::<Game>().announcements.is_empty());
}

#[test]
fn jumping_avoids_rim_contact_but_landing_restores_the_collision() {
    let mut app = app();
    app.add_systems(FixedUpdate, check_game);
    app.world_mut()
        .resource_mut::<powerups::Upgrades>()
        .jump_height = LANE_UNIT;
    app.world_mut().spawn(Flipper {
        lane: selected_lane(START_PROGRESS) as usize,
        depth: 0,
        motion: Motion::Pause(1),
        pause: 0,
    });
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(app.world().resource::<Game>().lives, 4);
    app.world_mut()
        .resource_mut::<powerups::Upgrades>()
        .jump_height = 0;
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().resource::<Game>().status == Status::Captured);
    finish_capture(&mut app);
    assert_eq!(app.world().resource::<Game>().lives, 3);
}

#[test]
fn capture_preserves_both_poses_then_retreats_the_camera_before_losing_a_life() {
    let mut app = app();
    app.init_resource::<ShotAssets>().add_systems(
        FixedUpdate,
        (
            move_glider.run_if(playing),
            powerups::update_jump,
            follow_glider.run_if(playing),
            fire_bullets.run_if(playing),
            move_enemies,
            check_game,
        )
            .chain(),
    );
    let player = app
        .world_mut()
        .query_filtered::<Entity, With<Glider>>()
        .single(app.world())
        .unwrap();
    let camera_start = Transform::from_translation(camera_position(
        glider_transform(START_PROGRESS).translation,
    ));
    let cameras = [0, 1].map(|_| {
        app.world_mut()
            .spawn((Camera3d::default(), camera_start))
            .id()
    });
    let captor = enemy(
        &mut app,
        7,
        0,
        Motion::Flip {
            from: 6,
            direction: 1,
            angle: 56,
        },
    );
    let other = enemy(&mut app, 2, 60 * LANE_UNIT, Motion::Climb);
    ticks(&mut app, 1);
    assert!(app.world().resource::<Game>().status == Status::Captured);
    let claw_start = *app.world().get::<Transform>(player).unwrap();
    let flipper_start = *app.world().get::<Transform>(captor).unwrap();
    let other_start = *app.world().get::<Transform>(other).unwrap();
    app.world_mut()
        .resource_mut::<powerups::Upgrades>()
        .jump_enabled = true;
    for key in [KeyCode::KeyA, KeyCode::KeyZ, KeyCode::KeyX] {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
    }
    ticks(&mut app, 79);
    for (entity, start) in [(player, claw_start), (captor, flipper_start)] {
        let transform = app.world().get::<Transform>(entity).unwrap();
        assert_eq!(
            transform.translation.truncate(),
            start.translation.truncate()
        );
        assert_eq!(transform.rotation, start.rotation);
        assert!(
            (transform.translation.z - (start.translation.z - 158.0 * WORLD_UNIT)).abs() < 1e-4
        );
    }
    assert_eq!(*app.world().get::<Transform>(other).unwrap(), other_start);
    assert_eq!(
        app.world().get::<Glider>(player).unwrap().progress,
        START_PROGRESS
    );
    assert_eq!(app.world().resource::<powerups::Upgrades>().jump_height, 0);
    assert_eq!(
        app.world_mut().query::<&Bullet>().iter(app.world()).count(),
        0
    );
    ticks(&mut app, 1);
    assert_eq!(app.world().get::<Flipper>(captor).unwrap().depth, REAR);
    assert_eq!(app.world().resource::<Game>().lives, 4);
    for camera in cameras {
        assert_eq!(*app.world().get::<Transform>(camera).unwrap(), camera_start);
    }
    ticks(&mut app, 1);
    for camera in cameras {
        assert!(
            (app.world().get::<Transform>(camera).unwrap().translation.z
                - camera_start.translation.z
                - 8.0 * WORLD_UNIT)
                .abs()
                < 1e-5
        );
    }
    ticks(&mut app, 48);
    assert_eq!(app.world().resource::<Game>().lives, 4);
    ticks(&mut app, 1);
    let game = app.world().resource::<Game>();
    assert!(game.status == Status::Entering);
    assert_eq!(game.lives, 3);
    assert_eq!(game.invulnerable, 120);
    assert!(game.capture.is_none());
    assert!(app.world().get_entity(captor).is_err());
    assert!(app.world().get_entity(other).is_err());
    assert_eq!(
        *app.world().get::<Transform>(player).unwrap(),
        glider_transform(START_PROGRESS)
    );
    for camera in cameras {
        assert_eq!(*app.world().get::<Transform>(camera).unwrap(), camera_start);
    }
}

#[test]
fn restarting_during_capture_restores_the_camera_and_cancels_the_animation() {
    let mut app = app();
    app.add_systems(FixedUpdate, check_game);
    let camera_start = camera_position(glider_transform(START_PROGRESS).translation);
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            Transform::from_translation(camera_start),
        ))
        .id();
    enemy(&mut app, 7, 0, Motion::Pause(0));
    ticks(&mut app, 82);
    assert!(app.world().resource::<Game>().status == Status::Captured);
    assert!(app.world().get::<Transform>(camera).unwrap().translation.z > camera_start.z);
    app.world_mut().resource_mut::<Game>().load = Some(LoadReason::Restart);
    ticks(&mut app, 1);
    assert!(app.world().resource::<Game>().status == Status::Entering);
    assert!(app.world().resource::<Game>().capture.is_none());
    assert_eq!(app.world().resource::<Game>().lives, 4);
    assert_eq!(
        app.world().get::<Transform>(camera).unwrap().translation,
        camera_start
    );
    ticks(&mut app, 150);
    assert_eq!(app.world().resource::<Game>().lives, 4);
}

#[test]
fn rim_contact_costs_a_life_resets_the_wave_and_grants_a_grace_period() {
    let mut app = app();
    app.add_systems(FixedUpdate, check_game);
    app.world_mut().resource_mut::<Game>().score = 300;
    enemy(&mut app, 7, 0, Motion::Pause(4));
    ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Game>().lives, 4);
    finish_capture(&mut app);
    let game = app.world().resource::<Game>();
    assert_eq!(game.lives, 3);
    assert_eq!(game.score, 300);
    assert_eq!(game.invulnerable, 120);
    ticks(&mut app, 256);
    enemy(&mut app, 7, 0, Motion::Pause(4));
    ticks(&mut app, 120);
    assert_eq!(app.world().resource::<Game>().lives, 3);
    ticks(&mut app, 1);
    finish_capture(&mut app);
    assert_eq!(app.world().resource::<Game>().lives, 2);
    ticks(&mut app, 256);
    app.world_mut().resource_mut::<Game>().invulnerable = 0;
    enemy(&mut app, 7, 0, Motion::Pause(4));
    ticks(&mut app, 1);
    finish_capture(&mut app);
    assert_eq!(app.world().resource::<Game>().lives, 1);
    assert!(app.world().resource::<Game>().status == Status::Entering);
    ticks(&mut app, 256);
    app.world_mut().resource_mut::<Game>().invulnerable = 0;
    enemy(&mut app, 7, 0, Motion::Pause(4));
    ticks(&mut app, 1);
    finish_capture(&mut app);
    assert!(app.world().resource::<Game>().status == Status::GameOver);
}

#[test]
fn completion_requires_no_remaining_spawns_or_enemies_but_does_not_wait_for_pickups() {
    let mut app = app();
    app.add_systems(FixedUpdate, check_game);
    app.world_mut().resource_mut::<Game>().remaining = 0;
    let id = enemy(&mut app, 1, REAR, Motion::Climb);
    ticks(&mut app, 1);
    assert!(app.world().resource::<Game>().status == Status::Playing);
    app.world_mut().despawn(id);
    powerups::spawn_drop(
        &mut app.world_mut().commands(),
        &powerups::PowerAssets::default(),
        1,
        REAR,
        &Level::default().web,
    );
    app.world_mut().flush();
    ticks(&mut app, 1);
    assert!(app.world().resource::<Game>().status == Status::Exiting);
    assert_eq!(
        app.world_mut()
            .query::<&powerups::PowerUp>()
            .iter(app.world())
            .count(),
        1
    );
}

#[test]
fn restart_clears_enemies_shots_score_and_game_over() {
    let mut app = app();
    app.world_mut().resource_mut::<Game>().status = Status::GameOver;
    app.world_mut().resource_mut::<Game>().score = 150;
    enemy(&mut app, 3, REAR, Motion::Climb);
    app.world_mut().resource_mut::<powerups::Upgrades>().laser = true;
    powerups::spawn_drop(
        &mut app.world_mut().commands(),
        &powerups::PowerAssets::default(),
        3,
        REAR,
        &Level::default().web,
    );
    app.world_mut().flush();
    app.world_mut().spawn(Bullet { lane: 2 });
    app.world_mut().resource_mut::<Game>().load = Some(LoadReason::Restart);
    ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Game>().score, 0);
    assert_eq!(app.world().resource::<Game>().lives, 4);
    assert!(!app.world().resource::<powerups::Upgrades>().laser);
    assert_eq!(
        app.world_mut()
            .query::<&powerups::PowerUp>()
            .iter(app.world())
            .count(),
        0
    );
    assert!(app.world().resource::<Game>().status == Status::Entering);
    assert!(
        app.world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .next()
            .is_none()
    );
    assert!(
        app.world_mut()
            .query::<&Bullet>()
            .iter(app.world())
            .next()
            .is_none()
    );
}
