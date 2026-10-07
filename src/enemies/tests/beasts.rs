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
fn horn_roots_meet_the_head_and_survive_as_identical_detached_parts() {
    use crate::enemies::beasts::{BeastAssets, setup};
    use bevy::mesh::VertexAttributeValues;
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>().add_systems(
        Startup,
        |mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>| {
            setup(&mut commands, &mut meshes)
        },
    );
    app.update();
    let assets = app.world().resource::<BeastAssets>();
    let meshes = app.world().resource::<Assets<Mesh>>();
    let positions = |handle: &Handle<Mesh>| {
        let Some(VertexAttributeValues::Float32x3(positions)) = meshes
            .get(handle)
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("Beastie meshes must have three-dimensional positions");
        };
        positions
    };
    let head = positions(&assets.frames[0]);
    for (index, root) in [Vec2::new(-0.3, 0.6), Vec2::new(0.3, 0.6)]
        .into_iter()
        .enumerate()
    {
        let horn = positions(&assets.parts[index]);
        let contains_root = |points: &[[f32; 3]]| {
            points
                .iter()
                .any(|p| Vec2::new(p[0], p[1]).distance(root) < 1e-6)
        };
        assert!(contains_root(head), "head must reach horn root {index}");
        assert!(contains_root(horn), "horn {index} must reach the head");
        let assembled = positions(&assets.frames[index + 1]);
        assert!(
            assembled.ends_with(horn),
            "detached horn must retain its assembled position"
        );
    }
    assert_eq!(head.len(), 24);
    assert_eq!(positions(&assets.frames[1]).len(), 36);
    assert_eq!(positions(&assets.frames[2]).len(), 48);
}

#[test]
fn level_49_generates_exactly_seventeen_armored_beasts_without_normal_flippers() {
    use crate::enemies::beasts::spawn;
    use crate::enemies::flippers::spawn_enemies;
    use crate::game::{Status, reset_wave};
    let mut app = crate::test_support::app();
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/49.toml")),
        "level49",
    )
    .unwrap();
    reset_wave(&mut app.world_mut().resource_mut::<Game>(), &level);
    app.world_mut().resource_mut::<Game>().status = Status::Playing;
    app.insert_resource(level)
        .add_systems(FixedUpdate, (spawn_enemies, spawn).chain());
    crate::test_support::ticks(&mut app, 70);
    assert_eq!(
        app.world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .count(),
        0
    );
    for remaining in (0..17).rev() {
        crate::test_support::ticks(&mut app, 1);
        let (id, beast, flipper) = app
            .world_mut()
            .query::<(Entity, &Beast, &Flipper)>()
            .single(app.world())
            .unwrap();
        assert_eq!(beast.stage, 2);
        assert!(matches!(flipper.motion, Motion::Approach));
        assert_eq!(app.world().resource::<Game>().remaining, remaining);
        app.world_mut().despawn(id);
        crate::test_support::ticks(&mut app, 140);
        assert_eq!(
            app.world_mut()
                .query::<&Flipper>()
                .iter(app.world())
                .count(),
            0
        );
    }
    crate::test_support::ticks(&mut app, 300);
    assert_eq!(
        app.world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(app.world().resource::<Game>().spawn_timer, 70);
}

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
