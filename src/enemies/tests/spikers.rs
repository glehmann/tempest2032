//! Checks Spiker growth, spike trimming, spawn limits, completion, and exit collisions.

use crate::animations::level_exit as exit;
use crate::collisions::resolve_hits;
use crate::enemies::spikers::Spike;
use crate::enemies::spikers::Spiker;
use crate::enemies::spikers::SpikerAssets;
use crate::enemies::spikers::{Motion, advance, spawn};
use crate::game::Game;
use crate::game::Status;
use crate::game::check_game;
use crate::game::reset_wave;
use crate::level::Level;
use crate::powerups;
use crate::test_support::ticks;
use crate::weapons::Bullet;
use crate::weapons::ParticleShot;
use crate::web::LANE_UNIT;
use crate::web::WORLD_UNIT;
use bevy::prelude::*;

#[test]
fn level_58_primary_dispatch_produces_spikers_and_counts_blocked_attempts() {
    use crate::enemies::flippers::{Flipper, spawn_enemies};
    let mut app = app();
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/58.toml")),
        "level58",
    )
    .unwrap();
    reset_wave(&mut app.world_mut().resource_mut::<Game>(), &level);
    app.world_mut().resource_mut::<Game>().status = Status::Playing;
    app.insert_resource(level)
        .add_systems(FixedUpdate, (spawn_enemies, spawn).chain());
    ticks(&mut app, 25);
    assert_eq!(
        app.world_mut().query::<&Spiker>().iter(app.world()).count(),
        0
    );
    ticks(&mut app, 1);
    assert_eq!(
        app.world_mut().query::<&Spiker>().iter(app.world()).count(),
        1
    );
    assert_eq!(app.world().resource::<Game>().remaining, 40);
    ticks(&mut app, 51);
    assert_eq!(
        app.world_mut().query::<&Spiker>().iter(app.world()).count(),
        1
    );
    assert_eq!(
        app.world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(app.world().resource::<Game>().remaining, 39);
}

#[test]
fn duplicate_generators_consume_both_attempts_but_cannot_spawn_two_spikers() {
    let mut app = app();
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/60.toml")),
        "level60",
    )
    .unwrap();
    reset_wave(&mut app.world_mut().resource_mut::<Game>(), &level);
    let mut game = app.world_mut().resource_mut::<Game>();
    assert_eq!(game.extra_spiker_timer, 750);
    game.status = Status::Playing;
    game.spiker_timer = 0;
    game.extra_spiker_timer = 0;
    app.insert_resource(level.clone())
        .add_systems(FixedUpdate, spawn);
    ticks(&mut app, 1);
    assert_eq!(
        app.world_mut().query::<&Spiker>().iter(app.world()).count(),
        1
    );
    assert_eq!(app.world().resource::<Game>().remaining, 49);
    assert_eq!(app.world().resource::<Game>().spiker_timer, 50);
    assert_eq!(app.world().resource::<Game>().extra_spiker_timer, 1500);
    reset_wave(&mut app.world_mut().resource_mut::<Game>(), &level);
    assert_eq!(app.world().resource::<Game>().extra_spiker_timer, 750);
    let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/60.toml"));
    assert!(
        Level::parse(&source.replace("[spikers]", "[beasts]"), "invalid")
            .err()
            .unwrap()
            .contains("extra_spikers")
    );
}

fn app() -> App {
    let mut app = crate::test_support::app();
    app.insert_resource(
        Level::parse(
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/04.toml")),
            "level4",
        )
        .unwrap(),
    )
    .init_resource::<SpikerAssets>();
    app
}

#[test]
fn integer_tip_comparison_extends_before_fractional_positions_cross() {
    let mut app = app();
    app.add_systems(FixedUpdate, advance);
    let spike = app
        .world_mut()
        .spawn(Spike {
            lane: 7,
            length: 30 * LANE_UNIT + 0xc000,
        })
        .id();
    app.world_mut().spawn((
        Spiker {
            lane: 7,
            depth: 130 * LANE_UNIT + 0x8000,
            motion: Motion::Climb,
            spike: Some(spike),
            build: 50,
            angle: 0,
            spike_speed: 0xd000,
        },
        Transform::default(),
    ));
    ticks(&mut app, 1);
    assert_eq!(
        app.world().get::<Spike>(spike).unwrap().length,
        30 * LANE_UNIT + 0xc000 + 0xd000
    );
}

#[test]
fn spiker_builds_fifty_one_extensions_then_returns_to_the_rear() {
    let mut app = app();
    app.add_systems(FixedUpdate, advance);
    let entity = app
        .world_mut()
        .spawn((
            Spiker {
                lane: 7,
                depth: 160 * LANE_UNIT,
                motion: Motion::Seek,
                spike: None,
                build: 50,
                angle: 0,
                spike_speed: 0xd000,
            },
            Transform::default(),
        ))
        .id();
    ticks(&mut app, 1);
    assert_eq!(
        app.world_mut()
            .query::<&Spike>()
            .single(app.world())
            .unwrap()
            .length,
        30 * LANE_UNIT
    );
    ticks(&mut app, 87);
    assert!(matches!(
        app.world().get::<Spiker>(entity).unwrap().motion,
        Motion::Descend
    ));
    assert_eq!(
        app.world_mut()
            .query::<&Spike>()
            .single(app.world())
            .unwrap()
            .length,
        30 * LANE_UNIT + 51 * 0xd000
    );
    assert_eq!(
        app.world().get::<Spiker>(entity).unwrap().depth,
        (160 * LANE_UNIT) - 87 * 0xd000
    );
    ticks(&mut app, 87);
    assert!(
        matches!(
            app.world().get::<Spiker>(entity).unwrap().motion,
            Motion::Seek
        ),
        "depth={}, motion={:?}",
        app.world().get::<Spiker>(entity).unwrap().depth,
        app.world().get::<Spiker>(entity).unwrap().motion
    );
    assert_eq!(
        app.world().get::<Spiker>(entity).unwrap().depth,
        160 * LANE_UNIT
    );
}

#[test]
fn shots_trim_spikes_by_three_units_and_leave_particle_shots_alive() {
    for particle in [false, true] {
        let mut app = app();
        app.add_systems(FixedUpdate, resolve_hits);
        let spike = app
            .world_mut()
            .spawn(Spike {
                lane: 7,
                length: 30 * LANE_UNIT,
            })
            .id();
        for _ in 0..11 {
            let length = app.world().get::<Spike>(spike).unwrap().length;
            let shot = app
                .world_mut()
                .spawn((
                    Bullet { lane: 7 },
                    Transform::from_xyz(
                        0.,
                        0.,
                        0.06 - (160. - length as f32 / LANE_UNIT as f32) * WORLD_UNIT,
                    ),
                ))
                .id();
            if particle {
                app.world_mut()
                    .entity_mut(shot)
                    .insert(ParticleShot::default());
            }
            ticks(&mut app, 1);
            assert_eq!(app.world().get_entity(shot).is_ok(), particle);
            if particle {
                app.world_mut().despawn(shot);
            }
        }
        assert!(app.world().get_entity(spike).is_err());
        assert_eq!(app.world().resource::<Game>().score, 36);
        assert_eq!(
            app.world_mut()
                .query::<&powerups::pickups::PowerUp>()
                .iter(app.world())
                .count(),
            0
        );
    }
}

#[test]
fn spikes_do_not_hold_the_wave_open_but_can_kill_the_claw_during_exit() {
    let mut app = app();
    app.add_systems(FixedUpdate, (exit::animate, check_game).chain());
    app.world_mut().resource_mut::<Game>().remaining = 0;
    app.world_mut().spawn(Spike {
        lane: 7,
        length: 150 * LANE_UNIT,
    });
    ticks(&mut app, 1);
    assert!(app.world().resource::<Game>().status == Status::Exiting);
    ticks(&mut app, 36);
    assert!(app.world().resource::<Game>().status == Status::Spiked);
    assert_eq!(app.world().resource::<Game>().lives, 4);
    ticks(&mut app, 100);
    assert_eq!(app.world().resource::<Game>().lives, 3);
    assert!(app.world().resource::<Game>().status == Status::Entering);
}

#[test]
fn a_living_spiker_blocks_completion_and_only_one_is_generated_at_a_time() {
    let mut app = app();
    let level = app.world().resource::<Level>().clone();
    reset_wave(&mut app.world_mut().resource_mut::<Game>(), &level);
    app.world_mut().resource_mut::<Game>().status = Status::Playing;
    app.world_mut().resource_mut::<Game>().entry = None;
    app.add_systems(FixedUpdate, spawn);
    ticks(&mut app, 400);
    assert_eq!(app.world().resource::<Game>().remaining, 25);
    ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Game>().remaining, 24);
    ticks(&mut app, 801);
    assert_eq!(app.world().resource::<Game>().remaining, 23);
    assert_eq!(
        app.world_mut().query::<&Spiker>().iter(app.world()).count(),
        1
    );
    app.world_mut().resource_mut::<Game>().remaining = 0;
    app.add_systems(FixedUpdate, check_game);
    ticks(&mut app, 1);
    assert!(app.world().resource::<Game>().status == Status::Playing);
}
