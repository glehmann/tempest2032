//! Checks Fuseball rail climbing, crossing timing, shot vulnerability, and rim-contact death.

use crate::enemies::fuseballs::FuseAssets;
use crate::enemies::fuseballs::Fuseball;
use crate::enemies::fuseballs::{Motion, advance};
use crate::game::Game;
use crate::game::Status;
use crate::glider::Glider;
use crate::level::Level;
use crate::powerups;
use crate::test_support::ticks;
use crate::weapons::Bullet;
use crate::weapons::Weapon;
use crate::web::LANE_UNIT;
use bevy::prelude::*;

fn app() -> App {
    let mut app = crate::test_support::app();
    app.insert_resource(
        Level::parse(
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/11.toml")),
            "level11",
        )
        .unwrap(),
    )
    .init_resource::<FuseAssets>()
    .add_systems(FixedUpdate, advance);
    app
}

#[test]
fn fuseballs_only_take_hits_in_the_middle_of_a_crossing() {
    for steps in [0, 3, 8, 11, 15] {
        let mut app = app();
        let entity = app
            .world_mut()
            .spawn((
                Fuseball {
                    depth: 60 * LANE_UNIT,
                    edge: 4,
                    motion: Motion::Cross {
                        from: 4,
                        to: 5,
                        lane: 4,
                        steps,
                        delay: 4,
                    },
                    rise: 48,
                    tick: 0,
                },
                Transform::default(),
                Mesh3d::default(),
                MeshMaterial3d::<StandardMaterial>::default(),
            ))
            .id();
        app.world_mut()
            .spawn((Bullet { lane: 4 }, Transform::from_xyz(0., 0., -10.)));
        ticks(&mut app, 1);
        assert_eq!(
            app.world().get_entity(entity).is_err(),
            (3..=11).contains(&steps)
        );
        if (3..=11).contains(&steps) {
            assert!([250, 500, 750].contains(&app.world().resource::<Game>().score));
        }
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
fn rail_rise_and_sixteen_five_tick_steps_match_the_original_counters() {
    let mut app = app();
    let entity = app
        .world_mut()
        .spawn((
            Fuseball {
                depth: 60 * LANE_UNIT,
                edge: 4,
                motion: Motion::Rail,
                rise: 48,
                tick: 0,
            },
            Transform::default(),
            Mesh3d::default(),
            MeshMaterial3d::<StandardMaterial>::default(),
        ))
        .id();
    ticks(&mut app, 48);
    assert!(matches!(
        app.world().get::<Fuseball>(entity).unwrap().motion,
        Motion::Rail
    ));
    ticks(&mut app, 1);
    assert!(matches!(
        app.world().get::<Fuseball>(entity).unwrap().motion,
        Motion::Cross { .. }
    ));
    assert_eq!(
        app.world().get::<Fuseball>(entity).unwrap().depth,
        60 * LANE_UNIT - 49 * 0x8000
    );
    ticks(&mut app, 79);
    assert!(matches!(
        app.world().get::<Fuseball>(entity).unwrap().motion,
        Motion::Cross { .. }
    ));
    ticks(&mut app, 1);
    assert!(matches!(
        app.world().get::<Fuseball>(entity).unwrap().motion,
        Motion::Rail
    ));
}

#[test]
fn a_rim_crossing_fries_the_claw_even_while_the_weapon_is_active() {
    let mut app = app();
    app.world_mut().spawn((
        Fuseball {
            depth: 0,
            edge: 7,
            motion: Motion::Cross {
                from: 7,
                to: 8,
                lane: 7,
                steps: 0,
                delay: 4,
            },
            rise: 48,
            tick: 0,
        },
        Transform::default(),
        Mesh3d::default(),
        MeshMaterial3d::<StandardMaterial>::default(),
    ));
    let entity = app
        .world_mut()
        .query_filtered::<Entity, With<Glider>>()
        .single(app.world())
        .unwrap();
    app.world_mut().get_mut::<Weapon>(entity).unwrap().firing = true;
    ticks(&mut app, 1);
    assert!(app.world().resource::<Game>().status == Status::Fried);
}
