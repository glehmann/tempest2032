//! Alien Droid ascent, recoil, traversal, lightning, protection, and child cleanup.

use super::{AlienAssets, AlienDroid, Motion, ZapBeam, advance, check_player, spawn};
use crate::{
    collisions::resolve_hits,
    game::{Game, Status, reset_wave},
    level::Level,
    powerups::Upgrades,
    weapons::Bullet,
    web::{LANE_UNIT, WORLD_UNIT},
};
use bevy::prelude::*;

fn app() -> App {
    let mut app = crate::test_support::app();
    app.insert_resource(
        Level::parse(
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/65.toml")),
            "level65",
        )
        .unwrap(),
    );
    app
}

fn alien(app: &mut App, lane: usize, depth: i32, motion: Motion) -> Entity {
    let alien = AlienDroid {
        lane,
        depth,
        motion,
        approach: false,
        direction: -1,
        angle: 0,
    };
    let pose = alien.transform(&app.world().resource::<Level>().web);
    app.world_mut()
        .spawn((
            alien,
            pose,
            Mesh3d::default(),
            MeshMaterial3d::<StandardMaterial>::default(),
        ))
        .id()
}

#[test]
fn crossing_takes_sixteen_three_tick_steps_then_zaps_for_thirty_two_ticks() {
    let mut app = app();
    app.add_systems(FixedUpdate, advance);
    let id = alien(&mut app, 0, -15 * LANE_UNIT, Motion::Ready);
    crate::test_support::ticks(&mut app, 1);
    assert!(matches!(
        app.world().get::<AlienDroid>(id).unwrap().motion,
        Motion::Cross {
            to: 15,
            step: 0,
            ..
        }
    ));
    crate::test_support::ticks(&mut app, 24);
    assert_eq!(app.world().get::<AlienDroid>(id).unwrap().lane, 0);
    crate::test_support::ticks(&mut app, 3);
    assert_eq!(app.world().get::<AlienDroid>(id).unwrap().lane, 15);
    crate::test_support::ticks(&mut app, 21);
    assert!(matches!(
        app.world().get::<AlienDroid>(id).unwrap().motion,
        Motion::Zap(32)
    ));
    crate::test_support::ticks(&mut app, 31);
    assert!(matches!(
        app.world().get::<AlienDroid>(id).unwrap().motion,
        Motion::Zap(1)
    ));
    crate::test_support::ticks(&mut app, 1);
    assert!(matches!(
        app.world().get::<AlienDroid>(id).unwrap().motion,
        Motion::Ready
    ));
}

#[test]
fn ascent_passes_the_rim_and_recoil_consumes_the_players_shot() {
    let mut app = app();
    app.add_systems(FixedUpdate, resolve_hits);
    let id = alien(&mut app, 5, 60 * LANE_UNIT, Motion::Climb);
    let shot = app
        .world_mut()
        .spawn((Bullet { lane: 5 }, Transform::from_xyz(0., 0., -10.)))
        .id();
    crate::test_support::ticks(&mut app, 1);
    assert!(app.world().get_entity(shot).is_err());
    assert_eq!(
        app.world().get::<AlienDroid>(id).unwrap().depth,
        82 * LANE_UNIT
    );
    assert_eq!(app.world().resource::<Game>().score, 0);
    app.add_systems(FixedUpdate, advance.after(resolve_hits));
    crate::test_support::ticks(&mut app, 72);
    assert!(!app.world().get::<AlienDroid>(id).unwrap().climbing());
    assert_eq!(
        app.world().get::<AlienDroid>(id).unwrap().depth,
        -15 * LANE_UNIT
    );
}

#[test]
fn lightning_frys_only_a_grounded_vulnerable_player_in_its_lane() {
    for (jump, invulnerable, expected) in [(0, 0, true), (1, 0, false), (0, 1, false)] {
        let mut app = app();
        app.world_mut().resource_mut::<Upgrades>().jump_height = jump;
        app.world_mut().resource_mut::<Game>().invulnerable = invulnerable;
        alien(&mut app, 7, -15 * LANE_UNIT, Motion::Zap(32));
        app.add_systems(FixedUpdate, check_player);
        crate::test_support::ticks(&mut app, 1);
        assert_eq!(
            app.world().resource::<Game>().status == Status::Fried,
            expected
        );
    }
}

#[test]
fn rim_phase_is_destroyed_by_one_hit_for_bonus_points_without_a_pickup() {
    let mut app = app();
    app.add_systems(FixedUpdate, resolve_hits);
    let id = alien(&mut app, 5, -15 * LANE_UNIT, Motion::Ready);
    app.world_mut().spawn((
        Bullet { lane: 5 },
        Transform::from_xyz(0., 0., 15. * WORLD_UNIT + 0.06),
    ));
    crate::test_support::ticks(&mut app, 1);
    assert!(app.world().get_entity(id).is_err());
    assert!([250, 500, 750].contains(&app.world().resource::<Game>().score));
    assert!(app.world_mut().resource_mut::<Upgrades>().drop_due(9));
}

#[test]
fn spawned_droids_share_the_budget_and_beams_are_removed_with_the_parent() {
    let mut app = app();
    let level = app.world().resource::<Level>().clone();
    reset_wave(&mut app.world_mut().resource_mut::<Game>(), &level);
    app.world_mut().resource_mut::<Game>().status = Status::Playing;
    app.add_systems(FixedUpdate, spawn);
    crate::test_support::ticks(&mut app, 350);
    assert_eq!(
        app.world_mut()
            .query::<&AlienDroid>()
            .iter(app.world())
            .count(),
        0
    );
    crate::test_support::ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Game>().remaining, 60);
    let id = app
        .world_mut()
        .query_filtered::<Entity, With<AlienDroid>>()
        .single(app.world())
        .unwrap();
    assert_eq!(
        app.world_mut()
            .query::<&ZapBeam>()
            .iter(app.world())
            .count(),
        1
    );
    app.world_mut().despawn(id);
    assert_eq!(
        app.world_mut()
            .query::<&ZapBeam>()
            .iter(app.world())
            .count(),
        0
    );
}

#[test]
fn graphics_have_the_original_eight_faces_and_thirty_two_beam_segments() {
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
    let assets = app.world().resource::<AlienAssets>();
    let meshes = app.world().resource::<Assets<Mesh>>();
    assert_eq!(meshes.get(&assets.body).unwrap().count_vertices(), 24);
    assert_eq!(meshes.get(&assets.beam).unwrap().count_vertices(), 192);
}
