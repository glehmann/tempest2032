//! Checks mixed enemy spawn budgets and the two Flippers released when a Tanker is shot.

use crate::collisions::resolve_hits;
use crate::enemies::flippers::Flipper;
use crate::enemies::flippers::Motion;
use crate::enemies::flippers::move_enemies;
use crate::enemies::flippers::spawn_enemies;
use crate::enemies::tankers::Tanker;
use crate::enemies::tankers::spawn;
use crate::game::Game;
use crate::game::Status;
use crate::game::reset_wave;
use crate::level::Level;
use crate::powerups;
use crate::weapons::Bullet;
use crate::web::LANE_UNIT;
use bevy::prelude::*;

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
