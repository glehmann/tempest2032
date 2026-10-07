//! Checks Beastie armor stripping and returning horns that absorb player shots.

use crate::collisions::resolve_hits;
use crate::enemies::beasts::Beast;
use crate::enemies::beasts::Reflected;
use crate::enemies::flippers::Flipper;
use crate::enemies::flippers::Motion;
use crate::enemies::shots::EnemyShot;
use crate::game::Game;
use crate::level::Level;
use crate::weapons::Bullet;
use crate::weapons::ShotSlot;
use crate::web::LANE_UNIT;
use bevy::prelude::*;

#[test]
fn three_rail_hits_strip_two_horns_then_kill_the_core() {
    let mut app = crate::test_support::app();
    app.insert_resource(
        Level::parse(
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/10.toml")),
            "level10",
        )
        .unwrap(),
    )
    .add_systems(FixedUpdate, resolve_hits);
    let beast = app
        .world_mut()
        .spawn((
            Beast { stage: 2 },
            Flipper {
                lane: 5,
                depth: 60 * LANE_UNIT,
                motion: Motion::Climb,
                pause: 0,
            },
            Transform::default(),
            Mesh3d::default(),
        ))
        .id();
    for stage in [1, 0] {
        app.world_mut()
            .spawn((Bullet { lane: 5 }, Transform::from_xyz(0., 0., -10.)));
        app.world_mut().run_schedule(FixedUpdate);
        assert_eq!(app.world().get::<Beast>(beast).unwrap().stage, stage);
        assert_eq!(app.world().resource::<Game>().score, 0);
    }
    assert_eq!(
        app.world_mut()
            .query::<&Reflected>()
            .iter(app.world())
            .count(),
        2
    );
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<ShotSlot>>()
            .iter(app.world())
            .count(),
        2
    );
    app.world_mut()
        .spawn((Bullet { lane: 5 }, Transform::from_xyz(0., 0., -10.)));
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().get_entity(beast).is_err());
    assert_eq!(app.world().resource::<Game>().score, 150);
}

#[test]
fn reflected_horns_absorb_normal_shots_without_disappearing() {
    let mut app = crate::test_support::app();
    app.add_systems(FixedUpdate, resolve_hits);
    let fragment = app
        .world_mut()
        .spawn((
            EnemyShot {
                lane: 3,
                depth: 60 * LANE_UNIT,
                angle: 0,
            },
            Reflected { speed: 0x1c000 },
            Transform::default(),
        ))
        .id();
    let shot = app
        .world_mut()
        .spawn((Bullet { lane: 3 }, Transform::from_xyz(0., 0., -10.)))
        .id();
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().get_entity(fragment).is_ok());
    assert!(app.world().get_entity(shot).is_err());
}
