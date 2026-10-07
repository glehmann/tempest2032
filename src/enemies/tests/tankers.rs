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

#[test]
fn tanker_children_can_mutate_into_spiral_flippers_without_fast_scatter() {
    use crate::enemies::flippers::SpiralFlipper;
    use crate::enemies::tankers::Released;
    let mut app = crate::test_support::app();
    let mut level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/75.toml")),
        "level75",
    )
    .unwrap();
    level.tankers.as_mut().unwrap().super_chance = 255;
    level.tankers.as_mut().unwrap().spiral_chance = 255;
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
    crate::test_support::ticks(&mut app, 1);
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<SpiralFlipper>>()
            .iter(app.world())
            .count(),
        2
    );
    assert!(
        app.world_mut()
            .query::<&Released>()
            .iter(app.world())
            .all(|r| !r.fast && r.remaining == 0)
    );
    app.add_systems(FixedUpdate, move_enemies.after(resolve_hits));
    crate::test_support::ticks(&mut app, 1);
    assert!(
        app.world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .all(|f| f.depth < 60 * LANE_UNIT)
    );
}

fn pulsar_level() -> Level {
    Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/33.toml")),
        "level33",
    )
    .unwrap()
}

#[test]
fn pulsar_tankers_spawn_without_direct_pulsars_and_reset_their_timer() {
    use crate::enemies::pulsars::{Pulsar, Pulse, pulse, spawn as spawn_pulsars};
    let mut app = crate::test_support::app();
    let level = pulsar_level();
    reset_wave(&mut app.world_mut().resource_mut::<Game>(), &level);
    app.world_mut().resource_mut::<Game>().status = Status::Playing;
    app.insert_resource(level.clone())
        .add_systems(FixedUpdate, (pulse, spawn, spawn_pulsars).chain());
    crate::test_support::ticks(&mut app, 250);
    assert_eq!(
        app.world_mut().query::<&Tanker>().iter(app.world()).count(),
        0
    );
    crate::test_support::ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Game>().remaining, 24);
    assert_eq!(
        *app.world_mut()
            .query::<&Contents>()
            .single(app.world())
            .unwrap(),
        Contents::Pulsars
    );
    assert_eq!(
        app.world_mut().query::<&Pulsar>().iter(app.world()).count(),
        0
    );
    assert_eq!(app.world().resource::<Pulse>().phase, 3);
    assert_eq!(app.world().resource::<Game>().pulsar_tanker_timer, 500);
    reset_wave(&mut app.world_mut().resource_mut::<Game>(), &level);
    assert_eq!(app.world().resource::<Game>().pulsar_tanker_timer, 250);
}

#[test]
fn pulsar_children_flip_once_before_climbing_or_electrifying_the_lane() {
    use crate::enemies::pulsars::{Pulsar, Pulse, advance};
    let mut app = crate::test_support::app();
    app.insert_resource(pulsar_level())
        .add_systems(FixedUpdate, resolve_hits);
    app.world_mut().spawn((
        Tanker {
            lane: 5,
            depth: 60 * LANE_UNIT,
            approach: false,
        },
        Contents::Pulsars,
        Transform::default(),
    ));
    app.world_mut()
        .spawn((Bullet { lane: 5 }, Transform::from_xyz(0., 0., -10.)));
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(app.world().resource::<Game>().score, 100);
    assert!(
        app.world_mut()
            .resource_mut::<powerups::Upgrades>()
            .drop_due(9)
    );
    let mut lanes = app
        .world_mut()
        .query::<&Pulsar>()
        .iter(app.world())
        .map(|p| {
            assert!(p.flipping());
            assert!(!p.electrified(7));
            assert_eq!(p.target(&app.world().resource::<Level>().web), None);
            p.lane
        })
        .collect::<Vec<_>>();
    lanes.sort();
    assert_eq!(lanes, [4, 6]);
    app.world_mut().resource_mut::<Pulse>().phase = 7;
    app.add_systems(FixedUpdate, advance);
    crate::test_support::ticks(&mut app, 27);
    assert!(
        app.world_mut()
            .query::<&Pulsar>()
            .iter(app.world())
            .all(|p| p.flipping() && p.depth == 60 * LANE_UNIT)
    );
    crate::test_support::ticks(&mut app, 1);
    assert!(
        app.world_mut()
            .query::<&Pulsar>()
            .iter(app.world())
            .all(|p| p.climbing() && p.electrified(7) && p.depth == 60 * LANE_UNIT)
    );
    crate::test_support::ticks(&mut app, 1);
    assert!(
        app.world_mut()
            .query::<&Pulsar>()
            .iter(app.world())
            .all(|p| p.depth == 60 * LANE_UNIT - 0x8000)
    );
}

#[test]
fn pulsar_tankers_release_two_flipping_children_at_the_rim() {
    use crate::enemies::pulsars::Pulsar;
    let mut app = crate::test_support::app();
    app.insert_resource(pulsar_level())
        .add_systems(FixedUpdate, move_tankers);
    app.world_mut().spawn((
        Tanker {
            lane: 5,
            depth: 1,
            approach: false,
        },
        Contents::Pulsars,
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
            .query::<&Pulsar>()
            .iter(app.world())
            .filter(|p| p.flipping() && p.depth == 0)
            .count(),
        2
    );
}

#[test]
fn all_three_tanker_generators_share_one_budget_and_children_keep_the_pulsar_speed() {
    use crate::enemies::pulsars::{Pulsar, advance};
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/41.toml")),
        "level41",
    )
    .unwrap();
    let mut app = crate::test_support::app();
    app.world_mut().resource_mut::<Game>().remaining = level.flippers.count;
    app.insert_resource(level).add_systems(FixedUpdate, spawn);
    crate::test_support::ticks(&mut app, 1);
    assert_eq!(
        app.world_mut().query::<&Tanker>().iter(app.world()).count(),
        3
    );
    assert_eq!(app.world().resource::<Game>().remaining, 38);
    let id = app
        .world_mut()
        .query::<(Entity, &Contents)>()
        .iter(app.world())
        .find(|(_, c)| **c == Contents::Pulsars)
        .unwrap()
        .0;
    let lane = app.world().get::<Tanker>(id).unwrap().lane;
    let mut tanker = app.world_mut().get_mut::<Tanker>(id).unwrap();
    tanker.approach = false;
    tanker.depth = 60 * LANE_UNIT;
    app.world_mut()
        .spawn((Bullet { lane }, Transform::from_xyz(0., 0., -10.)));
    app.add_systems(FixedUpdate, resolve_hits.after(spawn));
    crate::test_support::ticks(&mut app, 1);
    assert_eq!(
        app.world_mut().query::<&Pulsar>().iter(app.world()).count(),
        2
    );
    app.add_systems(FixedUpdate, advance.after(resolve_hits));
    crate::test_support::ticks(&mut app, 50);
    assert!(
        app.world_mut()
            .query::<&Pulsar>()
            .iter(app.world())
            .all(|p| p.climbing())
    );
    let depth = app
        .world_mut()
        .query::<&Pulsar>()
        .iter(app.world())
        .next()
        .unwrap()
        .depth;
    crate::test_support::ticks(&mut app, 1);
    assert_eq!(
        app.world_mut()
            .query::<&Pulsar>()
            .iter(app.world())
            .next()
            .unwrap()
            .depth,
        depth - 0x8000
    );
}

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
