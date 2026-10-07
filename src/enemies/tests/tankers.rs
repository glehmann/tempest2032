//! Checks mixed enemy spawn budgets and the two Flippers released when a Tanker is shot.

use crate::collisions::resolve_hits;
use crate::enemies::flippers::Flipper;
use crate::enemies::flippers::Motion;
use crate::enemies::flippers::move_enemies;
use crate::enemies::flippers::spawn_enemies;
use crate::enemies::fuseballs::{Fuseball, Motion as FuseMotion};
use crate::enemies::tankers::Tanker;
use crate::enemies::tankers::spawn;
use crate::enemies::tankers::{Contents, move_tankers};
use crate::game::Game;
use crate::game::Status;
use crate::game::reset_wave;
use crate::level::Level;
use crate::powerups;
use crate::weapons::Bullet;
use crate::web::LANE_UNIT;
use bevy::prelude::*;

fn fuse_level() -> Level {
    Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/25.toml")),
        "level25",
    )
    .unwrap()
}

#[test]
fn fuse_tankers_count_against_the_shared_budget_and_reset_their_timer() {
    let mut app = crate::test_support::app();
    let mut level = fuse_level();
    level.tankers = None;
    reset_wave(&mut app.world_mut().resource_mut::<Game>(), &level);
    assert_eq!(app.world().resource::<Game>().fuse_tanker_timer, 1000);
    app.world_mut().resource_mut::<Game>().status = Status::Playing;
    app.insert_resource(level.clone())
        .add_systems(FixedUpdate, spawn);
    crate::test_support::ticks(&mut app, 1000);
    assert_eq!(
        app.world_mut().query::<&Tanker>().iter(app.world()).count(),
        0
    );
    crate::test_support::ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Game>().remaining, 40);
    assert_eq!(app.world().resource::<Game>().fuse_tanker_timer, 2000);
    assert_eq!(
        *app.world_mut()
            .query::<&Contents>()
            .single(app.world())
            .unwrap(),
        Contents::Fuseballs
    );
    reset_wave(&mut app.world_mut().resource_mut::<Game>(), &level);
    assert_eq!(app.world().resource::<Game>().fuse_tanker_timer, 1000);
}

#[test]
fn shooting_fuse_tankers_sends_two_children_outward_at_the_same_depth() {
    for lane in [1, 11] {
        let mut app = crate::test_support::app();
        app.insert_resource(fuse_level())
            .add_systems(FixedUpdate, resolve_hits);
        app.world_mut().spawn((
            Tanker {
                lane,
                depth: 60 * LANE_UNIT,
                approach: false,
            },
            Contents::Fuseballs,
            Transform::default(),
        ));
        app.world_mut()
            .spawn((Bullet { lane }, Transform::from_xyz(0., 0., -10.)));
        app.world_mut().run_schedule(FixedUpdate);
        assert_eq!(app.world().resource::<Game>().score, 100);
        assert_eq!(app.world().resource::<Game>().remaining, 25);
        assert!(
            app.world_mut()
                .resource_mut::<powerups::Upgrades>()
                .drop_due(8)
        );
        assert_eq!(
            app.world_mut().query::<&Tanker>().iter(app.world()).count(),
            0
        );
        let mut crossings = app
            .world_mut()
            .query::<&Fuseball>()
            .iter(app.world())
            .map(|fuse| {
                assert_eq!(fuse.depth, 60 * LANE_UNIT);
                let FuseMotion::Cross {
                    from,
                    to,
                    lane,
                    steps,
                    delay,
                } = fuse.motion
                else {
                    panic!("released Fuseballs must cross immediately")
                };
                assert_eq!((steps, delay), (0, 4));
                (from, to, lane)
            })
            .collect::<Vec<_>>();
        crossings.sort();
        assert_eq!(
            crossings,
            [(lane, lane - 1, lane - 1), (lane + 1, lane + 2, lane + 1)]
        );
    }
}

#[test]
fn fuse_tankers_also_release_children_on_reaching_the_rim() {
    let mut app = crate::test_support::app();
    app.insert_resource(fuse_level())
        .add_systems(FixedUpdate, move_tankers);
    app.world_mut().spawn((
        Tanker {
            lane: 5,
            depth: 1,
            approach: false,
        },
        Contents::Fuseballs,
        Transform::default(),
        Mesh3d::default(),
        MeshMaterial3d::<StandardMaterial>::default(),
    ));
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(app.world().resource::<Game>().score, 100);
    assert_eq!(
        app.world_mut().query::<&Tanker>().iter(app.world()).count(),
        0
    );
    assert_eq!(
        app.world_mut()
            .query::<&Fuseball>()
            .iter(app.world())
            .filter(|f| f.depth == 0)
            .count(),
        2
    );
}

#[test]
fn fuse_tankers_reserve_capacity_for_their_children_and_retry_when_full() {
    let mut app = crate::test_support::app();
    let mut level = fuse_level();
    level.flippers.max_active = 3;
    app.insert_resource(level).add_systems(FixedUpdate, spawn);
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(app.world().resource::<Game>().remaining, 24);
    assert_eq!(app.world().resource::<Game>().fuse_tanker_timer, 0);
    let first = app
        .world_mut()
        .query_filtered::<Entity, With<Tanker>>()
        .single(app.world())
        .unwrap();
    app.world_mut().despawn(first);
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(app.world().resource::<Game>().remaining, 23);
    assert_eq!(
        *app.world_mut()
            .query::<&Contents>()
            .single(app.world())
            .unwrap(),
        Contents::Fuseballs
    );
}

#[test]
fn mixed_generators_share_the_original_seventeen_spawn_budget() {
    let mut app = crate::test_support::app();
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/03.toml")),
        "level3",
    )
    .unwrap();
    reset_wave(&mut app.world_mut().resource_mut::<Game>(), &level);
    app.world_mut().resource_mut::<Game>().status = Status::Playing;
    app.world_mut().resource_mut::<Game>().entry = None;
    app.insert_resource(level)
        .add_systems(FixedUpdate, (spawn_enemies, spawn).chain());
    for _ in 0..222 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert_eq!(
        app.world_mut().query::<&Tanker>().iter(app.world()).count(),
        0
    );
    assert_eq!(app.world().resource::<Game>().remaining, 15);
    app.world_mut().run_schedule(FixedUpdate);
    let tanker = app
        .world_mut()
        .query::<&Tanker>()
        .single(app.world())
        .unwrap();
    assert!((1..10).contains(&tanker.lane));
    assert_eq!(app.world().resource::<Game>().remaining, 14);
}

#[test]
fn shooting_a_tanker_releases_two_children_without_advancing_the_drop_counter() {
    let mut app = crate::test_support::app();
    let mut level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/03.toml")),
        "level3",
    )
    .unwrap();
    level.tankers.as_mut().unwrap().super_chance = 0;
    app.insert_resource(level)
        .add_systems(FixedUpdate, resolve_hits);
    app.world_mut().spawn((
        Tanker {
            lane: 5,
            depth: 60 * LANE_UNIT,
            approach: false,
        },
        Transform::default(),
    ));
    app.world_mut()
        .spawn((Bullet { lane: 5 }, Transform::from_xyz(0., 0., -10.)));
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(app.world().resource::<Game>().score, 100);
    assert_eq!(
        app.world_mut().query::<&Tanker>().iter(app.world()).count(),
        0
    );
    assert_eq!(
        app.world_mut()
            .query::<&powerups::PowerUp>()
            .iter(app.world())
            .count(),
        0
    );
    let mut lanes = app
        .world_mut()
        .query::<&Flipper>()
        .iter(app.world())
        .map(|f| f.lane)
        .collect::<Vec<_>>();
    lanes.sort();
    assert_eq!(lanes, [4, 6]);
    app.add_systems(FixedUpdate, move_enemies);
    for _ in 0..34 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert!(
        app.world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .all(|f| matches!(f.motion, Motion::Climb) && f.depth < 60 * LANE_UNIT)
    );
}
