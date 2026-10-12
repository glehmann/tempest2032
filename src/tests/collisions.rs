//! Checks firing protection at the rim, nearest hits, laser piercing, scoring, and shot interception.

use crate::collisions::resolve_hits;
use crate::enemies::flippers::Motion;
use crate::enemies::flippers::move_enemies;
use crate::enemies::flippers::spawn_enemies;
use crate::enemies::shots::EnemyShot;
use crate::enemies::shots::fire_enemy_shots;
use crate::enemies::shots::move_enemy_shots;
use crate::game::Game;
use crate::game::Status;
use crate::game::check_game;
use crate::game::playing;
use crate::powerups;
use crate::test_support::app;
use crate::test_support::enemy;
use crate::test_support::ticks;
use crate::weapons::Bullet;
use crate::weapons::FIRE_INTERVAL;
use crate::weapons::MAX_BULLETS;
use crate::weapons::ParticleShot;
use crate::weapons::ShotAssets;
use crate::weapons::fire_bullets;
use crate::weapons::move_bullets;
use crate::web::LANE_UNIT;
use crate::web::WORLD_UNIT;
use bevy::prelude::*;

#[test]
fn active_fire_covers_the_normal_shot_delay_but_releasing_fire_allows_a_grab() {
    for laser in [false, true] {
        for release in [false, true] {
            let mut app = app();
            app.init_resource::<ShotAssets>().add_systems(
                FixedUpdate,
                (
                    move_bullets,
                    fire_bullets,
                    move_enemies,
                    resolve_hits,
                    check_game,
                )
                    .chain(),
            );
            app.world_mut().resource_mut::<powerups::Upgrades>().laser = laser;
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Enter);
            ticks(&mut app, 2);
            let flipper = enemy(
                &mut app,
                7,
                0,
                Motion::Flip {
                    from: 6,
                    direction: 1,
                    angle: 56,
                },
            );
            ticks(&mut app, 1);
            assert_eq!(app.world().resource::<Game>().lives, 4);
            assert!(
                app.world().get_entity(flipper).is_ok(),
                "the latest shot is already beyond the rim collision range"
            );
            if release {
                app.world_mut()
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .release(KeyCode::Enter);
            }
            ticks(&mut app, if laser { 1 } else { 2 });
            assert_eq!(app.world().resource::<Game>().lives, 4);
            assert!(
                app.world().resource::<Game>().status
                    == if release {
                        Status::Captured
                    } else {
                        Status::Playing
                    }
            );
            assert_eq!(
                app.world().resource::<Game>().score,
                if release { 0 } else { 150 }
            );
        }
    }
}

#[test]
fn a_full_magazine_allows_a_grab_even_with_enter_held() {
    let mut app = app();
    app.init_resource::<ShotAssets>().add_systems(
        FixedUpdate,
        (
            move_bullets,
            fire_bullets,
            move_enemies,
            resolve_hits,
            check_game,
        )
            .chain(),
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    for _ in 0..MAX_BULLETS {
        app.world_mut()
            .spawn((Bullet { lane: 0 }, Transform::from_xyz(0.0, 0.0, -10.0)));
    }
    enemy(
        &mut app,
        7,
        0,
        Motion::Flip {
            from: 6,
            direction: 1,
            angle: 56,
        },
    );
    ticks(&mut app, 1);
    assert!(app.world().resource::<Game>().status == Status::Captured);
}

#[test]
fn holding_fire_without_moving_cannot_clear_the_first_wave() {
    for laser in [false, true] {
        let mut app = app();
        app.world_mut().resource_mut::<powerups::Upgrades>().laser = laser;
        app.init_resource::<ShotAssets>().add_systems(
            FixedUpdate,
            (
                move_bullets,
                fire_bullets.run_if(playing),
                spawn_enemies,
                move_enemies,
                move_enemy_shots,
                resolve_hits,
                fire_enemy_shots,
                powerups::advance_powerups,
                check_game,
            )
                .chain(),
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        for _ in 0..15_000 {
            ticks(&mut app, 1);
            if app.world().resource::<Game>().lives < 4 {
                break;
            }
            assert!(
                app.world().resource::<Game>().status != Status::Exiting,
                "stationary held fire cleared the wave"
            );
        }
        assert!(
            app.world().resource::<Game>().lives < 4,
            "stationary held fire survived the entire wave, laser={laser}"
        );
    }
}

#[test]
fn firing_destroys_an_incoming_flipper_before_it_can_grab_the_player() {
    for laser in [false, true] {
        let mut app = app();
        app.init_resource::<ShotAssets>().add_systems(
            FixedUpdate,
            (
                move_bullets,
                fire_bullets,
                move_enemies,
                resolve_hits,
                check_game,
            )
                .chain(),
        );
        app.world_mut().resource_mut::<powerups::Upgrades>().laser = laser;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        enemy(
            &mut app,
            7,
            0,
            Motion::Flip {
                from: 6,
                direction: 1,
                angle: 56,
            },
        );
        ticks(&mut app, 1);
        assert_eq!(app.world().resource::<Game>().lives, 4);
        assert_eq!(app.world().resource::<Game>().score, 150);
    }
}

#[test]
fn a_pause_in_fire_lets_an_incoming_flipper_grab_the_player() {
    let mut app = app();
    app.init_resource::<ShotAssets>().add_systems(
        FixedUpdate,
        (
            move_bullets,
            fire_bullets,
            move_enemies,
            resolve_hits,
            check_game,
        )
            .chain(),
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    enemy(
        &mut app,
        7,
        0,
        Motion::Flip {
            from: 6,
            direction: 1,
            angle: 0,
        },
    );
    ticks(&mut app, 8);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::Enter);
    ticks(&mut app, 24);
    assert!(app.world().resource::<Game>().status == Status::Captured);
    assert_eq!(app.world().resource::<Game>().score, 0);
}

#[test]
fn shots_at_the_rim_leave_a_gap_between_normal_fire_intervals() {
    let mut app = app();
    app.add_systems(FixedUpdate, resolve_hits);
    let flipper = enemy(&mut app, 7, 0, Motion::Pause(4));
    for depth in [6.5, 6.5 + 3.5 * f32::from(FIRE_INTERVAL)] {
        app.world_mut().spawn((
            Bullet { lane: 7 },
            Transform::from_xyz(0.0, 0.0, 0.06 - depth * WORLD_UNIT),
        ));
    }
    ticks(&mut app, 1);
    assert!(app.world().get_entity(flipper).is_ok());
    app.world_mut().spawn((
        Bullet { lane: 7 },
        Transform::from_xyz(0.0, 0.0, 0.06 - 5.5 * WORLD_UNIT),
    ));
    ticks(&mut app, 1);
    assert!(app.world().get_entity(flipper).is_err());
}

#[test]
fn particle_shots_pass_through_multiple_enemies() {
    let mut app = app();
    app.add_systems(FixedUpdate, resolve_hits);
    let near = enemy(&mut app, 3, 60 * LANE_UNIT, Motion::Climb);
    let far = enemy(&mut app, 3, 64 * LANE_UNIT, Motion::Climb);
    let shot = app
        .world_mut()
        .spawn((
            Bullet { lane: 3 },
            ParticleShot::default(),
            Transform::from_xyz(0.0, 0.0, -10.3),
        ))
        .id();
    ticks(&mut app, 1);
    assert!(app.world().get_entity(near).is_err());
    assert!(app.world().get_entity(far).is_err());
    assert!(app.world().get_entity(shot).is_ok());
    assert_eq!(app.world().resource::<Game>().score, 300);
}

#[test]
fn shots_hit_only_their_lane_and_kill_one_nearest_enemy_for_150_points() {
    let mut app = app();
    app.add_systems(FixedUpdate, resolve_hits);
    let near = enemy(&mut app, 3, 60 * LANE_UNIT, Motion::Climb);
    let far = enemy(&mut app, 3, 64 * LANE_UNIT, Motion::Climb);
    let other = enemy(&mut app, 4, 60 * LANE_UNIT, Motion::Climb);
    let bullet = app
        .world_mut()
        .spawn((Bullet { lane: 3 }, Transform::from_xyz(0.0, 0.0, -10.3)))
        .id();
    ticks(&mut app, 1);
    assert!(app.world().get_entity(near).is_err());
    assert!(app.world().get_entity(bullet).is_err());
    assert!(app.world().get_entity(far).is_ok());
    assert!(app.world().get_entity(other).is_ok());
    assert_eq!(app.world().resource::<Game>().score, 150);
}

#[test]
fn duplicate_shots_cannot_award_the_same_kill_twice() {
    let mut app = app();
    app.add_systems(FixedUpdate, resolve_hits);
    enemy(&mut app, 2, 60 * LANE_UNIT, Motion::Climb);
    for _ in 0..2 {
        app.world_mut()
            .spawn((Bullet { lane: 2 }, Transform::from_xyz(0.0, 0.0, -10.0)));
    }
    ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Game>().score, 150);
    assert_eq!(
        app.world_mut()
            .query::<&powerups::PowerUp>()
            .iter(app.world())
            .count(),
        1
    );
    assert!(
        !app.world().resource::<powerups::Upgrades>().laser,
        "a drop must be collected first"
    );
    assert_eq!(
        app.world_mut().query::<&Bullet>().iter(app.world()).count(),
        1
    );
}

#[test]
fn enemy_shots_are_interceptable_and_do_not_award_flipper_points() {
    let mut app = app();
    app.add_systems(FixedUpdate, resolve_hits);
    let shot = app
        .world_mut()
        .spawn(EnemyShot {
            lane: 1,
            depth: 60 * LANE_UNIT,
            angle: 0,
        })
        .id();
    app.world_mut()
        .spawn((Bullet { lane: 1 }, Transform::from_xyz(0., 0., -10.)));
    ticks(&mut app, 1);
    assert!(app.world().get_entity(shot).is_err());
    assert_eq!(app.world().resource::<Game>().score, 0);
}
