//! Mirror approach, stopping distance, shot reflection, durability, and spawn budgets.

use super::{Mirror, MirrorAssets, ReturnedShot, advance, spawn};
use crate::{
    collisions::resolve_hits,
    enemies::{
        flippers::{Flipper, Motion},
        shots::{EnemyShot, move_enemy_shots},
    },
    game::Game,
    level::Level,
    powerups::Upgrades,
    weapons::{Bullet, ParticleShot, ShotSlot},
    web::{LANE_UNIT, WORLD_UNIT},
};
use bevy::prelude::*;

fn app() -> App {
    let mut app = crate::test_support::app();
    app.insert_resource(
        Level::parse(
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/57.toml")),
            "level57",
        )
        .unwrap(),
    );
    app
}

fn mirror(app: &mut App, depth: i32, approach: bool) -> Entity {
    let mirror = Mirror {
        lane: 5,
        depth,
        approach,
        hits: 5,
        angle: 0,
    };
    let pose = mirror.transform(&app.world().resource::<Level>().web);
    app.world_mut()
        .spawn((
            mirror,
            pose,
            Mesh3d::default(),
            MeshMaterial3d::<StandardMaterial>::default(),
        ))
        .id()
}

fn bullet(app: &mut App, depth: i32, particle: bool) -> Entity {
    let mut shot = app.world_mut().spawn((
        Bullet { lane: 5 },
        Transform::from_xyz(0., 0., -depth as f32 / LANE_UNIT as f32 * WORLD_UNIT + 0.06),
    ));
    if particle {
        shot.insert(ParticleShot::default());
    }
    shot.id()
}

#[test]
fn mirrors_stop_forty_units_from_the_rim_and_resume_after_recoil() {
    let mut app = app();
    app.add_systems(FixedUpdate, advance);
    let id = mirror(&mut app, 162 * LANE_UNIT, true);
    crate::test_support::ticks(&mut app, 1);
    assert!(!app.world().get::<Mirror>(id).unwrap().approach);
    crate::test_support::ticks(&mut app, 320);
    assert_eq!(app.world().get::<Mirror>(id).unwrap().depth, 40 * LANE_UNIT);
    assert!(
        !app.world_mut()
            .get_mut::<Mirror>(id)
            .unwrap()
            .hit(0xc000, 160 * LANE_UNIT)
    );
    assert_eq!(app.world().get::<Mirror>(id).unwrap().depth, 52 * LANE_UNIT);
    crate::test_support::ticks(&mut app, 1);
    assert_eq!(
        app.world().get::<Mirror>(id).unwrap().depth,
        52 * LANE_UNIT - 0xc000
    );
}

#[test]
fn surviving_hits_return_the_shot_at_full_speed_and_keep_its_magazine_slot() {
    for particle in [false, true] {
        let mut app = app();
        app.add_systems(FixedUpdate, resolve_hits);
        let id = mirror(&mut app, 60 * LANE_UNIT, false);
        let shot = bullet(&mut app, 60 * LANE_UNIT, particle);
        app.world_mut().run_schedule(FixedUpdate);
        assert!(app.world().get_entity(shot).is_err());
        assert_eq!(app.world().get::<Mirror>(id).unwrap().hits, 4);
        assert_eq!(app.world().get::<Mirror>(id).unwrap().depth, 72 * LANE_UNIT);
        let (returned_id, returned) = app
            .world_mut()
            .query::<(Entity, &ReturnedShot)>()
            .single(app.world())
            .unwrap();
        assert_eq!(returned.speed, if particle { 0x48000 } else { 0x38000 });
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<ShotSlot>>()
                .iter(app.world())
                .count(),
            1
        );
        app.add_systems(FixedUpdate, move_enemy_shots.after(resolve_hits));
        crate::test_support::ticks(&mut app, 1);
        assert_eq!(
            app.world().get::<EnemyShot>(returned_id).unwrap().depth,
            60 * LANE_UNIT - if particle { 0x48000 } else { 0x38000 }
        );
        assert_eq!(app.world().resource::<Game>().score, 0);
    }
}

#[test]
fn reflected_particle_shots_cannot_pierce_an_enemy_behind_the_mirror() {
    let mut app = app();
    app.add_systems(FixedUpdate, resolve_hits);
    mirror(&mut app, 60 * LANE_UNIT, false);
    let enemy = crate::test_support::enemy(&mut app, 5, 63 * LANE_UNIT, Motion::Climb);
    bullet(&mut app, 60 * LANE_UNIT, true);
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().get::<Flipper>(enemy).is_some());
    assert_eq!(
        app.world_mut()
            .query::<&ReturnedShot>()
            .iter(app.world())
            .count(),
        1
    );
}

#[test]
fn the_fifth_hit_destroys_the_mirror_with_bonus_points_and_no_pickup() {
    let mut app = app();
    app.add_systems(FixedUpdate, resolve_hits);
    let id = mirror(&mut app, 40 * LANE_UNIT, false);
    let mut last = Entity::PLACEHOLDER;
    for _ in 0..5 {
        let depth = app.world().get::<Mirror>(id).unwrap().depth;
        last = bullet(&mut app, depth, false);
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert!(app.world().get_entity(id).is_err());
    assert!(app.world().get::<Bullet>(last).is_some());
    assert!([250, 500, 750].contains(&app.world().resource::<Game>().score));
    assert!(app.world_mut().resource_mut::<Upgrades>().drop_due(9));
}

#[test]
fn a_mirror_pushed_behind_the_rear_dies_without_reflecting_the_hit() {
    let mut app = app();
    app.add_systems(FixedUpdate, resolve_hits);
    let id = mirror(&mut app, 159 * LANE_UNIT, false);
    bullet(&mut app, 159 * LANE_UNIT, false);
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().get_entity(id).is_err());
    assert_eq!(
        app.world_mut()
            .query::<&ReturnedShot>()
            .iter(app.world())
            .count(),
        0
    );
}

#[test]
fn mirrors_count_toward_other_generators_capacity_and_share_the_wave_budget() {
    use crate::enemies::flippers::spawn_enemies;
    let mut app = app();
    app.world_mut().resource_mut::<Level>().flippers.max_active = 1;
    mirror(&mut app, 60 * LANE_UNIT, false);
    app.add_systems(FixedUpdate, (spawn_enemies, spawn).chain());
    crate::test_support::ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Game>().remaining, 25);
    assert_eq!(
        app.world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .count(),
        0
    );
    let id = app
        .world_mut()
        .query_filtered::<Entity, With<Mirror>>()
        .single(app.world())
        .unwrap();
    app.world_mut().despawn(id);
    crate::test_support::ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Game>().remaining, 24);
}

#[test]
fn mirrors_first_spawn_after_two_hundred_and_one_ticks() {
    use crate::game::{Status, reset_wave};
    let mut app = app();
    let level = app.world().resource::<Level>().clone();
    reset_wave(&mut app.world_mut().resource_mut::<Game>(), &level);
    app.world_mut().resource_mut::<Game>().status = Status::Playing;
    app.add_systems(FixedUpdate, spawn);
    crate::test_support::ticks(&mut app, 200);
    assert_eq!(
        app.world_mut().query::<&Mirror>().iter(app.world()).count(),
        0
    );
    crate::test_support::ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Game>().remaining, 60);
    assert_eq!(app.world().resource::<Game>().mirror_timer, 400);
    assert_eq!(
        app.world_mut()
            .query::<&Mirror>()
            .single(app.world())
            .unwrap()
            .hits,
        5
    );
}

#[test]
fn mirror_graphics_use_the_original_six_faces() {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .add_systems(
            Startup,
            |mut commands: Commands,
             mut meshes: ResMut<Assets<Mesh>>,
             mut materials: ResMut<Assets<StandardMaterial>>| {
                super::setup(&mut commands, &mut meshes, &mut materials)
            },
        );
    app.update();
    let assets = app.world().resource::<MirrorAssets>();
    assert_eq!(
        app.world()
            .resource::<Assets<Mesh>>()
            .get(&assets.body)
            .unwrap()
            .count_vertices(),
        18
    );
}
