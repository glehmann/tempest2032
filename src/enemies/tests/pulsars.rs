//! Checks Pulsar phase synchronization, safe touchdown, rim sparks, shooting,
//! lane electricity, jumping, shared generation limits, and retry resets.

use crate::enemies::pulsars::check_sparks;
use crate::{
    collisions::resolve_hits,
    enemies::{
        flippers::{Flipper, Motion},
        pulsar_render::PulsarAssets,
        pulsars::{Pulsar, Pulse, Spark, advance, check_player, pulse, spawn},
    },
    game::{Game, Status},
    level::{Level, Levels, LoadReason},
    powerups::{PowerUp, Upgrades},
    test_support::ticks,
    weapons::{Bullet, Weapon},
    web::{LANE_UNIT, WORLD_UNIT},
};
use bevy::prelude::*;

fn app() -> App {
    let mut app = crate::test_support::app();
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/17.toml")),
        "level17",
    )
    .unwrap();
    app.world_mut().resource_mut::<Game>().pulsar_timer =
        level.pulsars.as_ref().unwrap().generator.first_spawn_ticks - 1;
    app.insert_resource(level)
        .init_resource::<Pulse>()
        .insert_resource(PulsarAssets {
            shades: vec![Handle::default(); 256],
            ..default()
        });
    app
}

fn enemy(app: &mut App, lane: usize, depth: i32, approach: bool) -> Entity {
    let mut pose = app
        .world()
        .resource::<Level>()
        .web
        .glider_transform(lane as i32 * LANE_UNIT);
    pose.translation.z = -depth as f32 / LANE_UNIT as f32 * WORLD_UNIT + 0.06;
    app.world_mut()
        .spawn((
            Pulsar {
                lane,
                depth,
                approach,
                spark: None,
                flip: None,
            },
            pose,
        ))
        .id()
}

#[test]
fn shared_phase_advances_every_nine_ticks_and_rear_touchdown_waits_for_safety() {
    let mut app = app();
    app.add_systems(FixedUpdate, (pulse, advance).chain());
    ticks(&mut app, 62);
    assert_eq!(app.world().resource::<Pulse>().phase, 6);
    ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Pulse>().phase, 7);
    let id = enemy(&mut app, 0, 160 * LANE_UNIT + 2 * LANE_UNIT, true);
    ticks(&mut app, 1);
    assert!(app.world().get::<Pulsar>(id).unwrap().approach);
    assert_eq!(
        app.world().get::<Pulsar>(id).unwrap().depth,
        160 * LANE_UNIT
    );
    app.world_mut().resource_mut::<Pulse>().phase = 0;
    ticks(&mut app, 1);
    assert!(!app.world().get::<Pulsar>(id).unwrap().approach);
    assert_eq!(
        app.world().get::<Pulsar>(id).unwrap().depth,
        160 * LANE_UNIT - 0x8000
    );
    ticks(&mut app, 1);
    assert_eq!(
        app.world().get::<Pulsar>(id).unwrap().depth,
        159 * LANE_UNIT
    );
}

#[test]
fn reaching_the_rim_releases_two_sparks_with_five_tick_propagation_and_wrapping() {
    let mut app = app();
    app.add_systems(FixedUpdate, advance);
    enemy(&mut app, 0, LANE_UNIT / 4, false);
    ticks(&mut app, 1);
    assert_eq!(
        app.world_mut().query::<&Pulsar>().iter(app.world()).count(),
        2
    );
    ticks(&mut app, 4);
    assert!(
        app.world_mut()
            .query::<&Pulsar>()
            .iter(app.world())
            .all(|p| p.lane == 0)
    );
    ticks(&mut app, 1);
    let mut lanes = app
        .world_mut()
        .query::<&Pulsar>()
        .iter(app.world())
        .map(|p| p.lane)
        .collect::<Vec<_>>();
    lanes.sort();
    assert_eq!(lanes, [1, 17]);
}

#[test]
fn electricity_ignores_firing_but_respects_jump_and_invulnerability() {
    for (jump, grace, dies) in [(0, 0, true), (LANE_UNIT, 0, false), (0, 1, false)] {
        let mut app = app();
        app.add_systems(FixedUpdate, check_player);
        app.world_mut().resource_mut::<Pulse>().phase = 7;
        app.world_mut().resource_mut::<Game>().invulnerable = grace;
        app.world_mut().resource_mut::<Upgrades>().jump_height = jump;
        app.world_mut()
            .query::<&mut Weapon>()
            .single_mut(app.world_mut())
            .unwrap()
            .firing = true;
        enemy(&mut app, 7, 100 * LANE_UNIT, false);
        ticks(&mut app, 1);
        assert_eq!(app.world().resource::<Game>().status == Status::Fried, dies);
    }
    let mut app = app();
    app.add_systems(
        FixedUpdate,
        (check_sparks, resolve_hits, check_player).chain(),
    );
    let id = enemy(&mut app, 7, 0, false);
    app.world_mut().get_mut::<Pulsar>(id).unwrap().spark = Some(Spark {
        direction: 1,
        ticks: 5,
    });
    let pose = *app.world().get::<Transform>(id).unwrap();
    app.world_mut().spawn((Bullet { lane: 7 }, pose));
    ticks(&mut app, 1);
    assert!(app.world().resource::<Game>().status == Status::Fried);
    assert!(
        app.world().get::<Pulsar>(id).is_some(),
        "rim contact precedes shot collision, as in run_pspark"
    );
}

#[test]
fn sparks_bounce_inward_at_both_open_web_endpoints() {
    for (lane, expected) in [(0, 1), (16, 15)] {
        let mut app = app();
        let mut level = app.world_mut().resource_mut::<Level>();
        level.web.closed = false;
        level.web.orientations.pop();
        app.add_systems(FixedUpdate, advance);
        enemy(&mut app, lane, LANE_UNIT / 4, false);
        ticks(&mut app, 6);
        assert!(
            app.world_mut()
                .query::<&Pulsar>()
                .iter(app.world())
                .all(|p| p.lane == expected)
        );
    }
}

#[test]
fn rail_shots_award_two_hundred_points_and_nearest_hit_selection_stays_intact() {
    let mut app = app();
    app.add_systems(FixedUpdate, resolve_hits);
    let pulsar = enemy(&mut app, 0, 60 * LANE_UNIT, false);
    let flipper = crate::test_support::enemy(&mut app, 0, 62 * LANE_UNIT, Motion::Climb);
    let position = *app.world().get::<Transform>(pulsar).unwrap();
    app.world_mut().spawn((Bullet { lane: 0 }, position));
    ticks(&mut app, 1);
    assert!(app.world().get_entity(pulsar).is_err());
    assert!(app.world().get::<Flipper>(flipper).is_some());
    assert_eq!(app.world().resource::<Game>().score, 200);
    assert_eq!(
        app.world_mut()
            .query::<&PowerUp>()
            .iter(app.world())
            .count(),
        1
    );
    let spark = enemy(&mut app, 1, 0, false);
    app.world_mut().get_mut::<Pulsar>(spark).unwrap().spark = Some(Spark {
        direction: 1,
        ticks: 5,
    });
    let pose = *app.world().get::<Transform>(spark).unwrap();
    app.world_mut().spawn((Bullet { lane: 1 }, pose));
    ticks(&mut app, 1);
    assert!(app.world().get_entity(spark).is_err());
    assert_eq!(app.world().resource::<Game>().score, 350);
}

#[test]
fn generator_uses_the_shared_budget_and_retry_resets_the_pulse() {
    let mut app = app();
    app.add_systems(FixedUpdate, spawn);
    ticks(&mut app, 300);
    assert_eq!(
        app.world_mut().query::<&Pulsar>().iter(app.world()).count(),
        0
    );
    ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Game>().remaining, 24);
    assert_eq!(app.world().resource::<Game>().pulsar_timer, 600);
    for lane in 0..10 {
        enemy(&mut app, lane, 50 * LANE_UNIT, false);
    }
    app.world_mut().resource_mut::<Game>().pulsar_timer = 0;
    ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Game>().remaining, 24);
    app.insert_resource(
        Levels::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("levels")).unwrap(),
    );
    app.world_mut().resource_mut::<Game>().level_index = 16;
    app.world_mut().resource_mut::<Game>().load = Some(LoadReason::Retry);
    app.world_mut().resource_mut::<Pulse>().phase = 7;
    ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Pulse>().phase, 0);
    assert_eq!(
        app.world_mut().query::<&Pulsar>().iter(app.world()).count(),
        0
    );
    assert_eq!(app.world().resource::<Game>().pulsar_timer, 300);
}
