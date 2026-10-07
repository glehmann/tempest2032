//! Checks pickup formation, lane and height requirements, rewards, and collection-burst cleanup.

use crate::enemies::flipper_render::FlipperAssets;
use crate::enemies::tankers;
use crate::explosion;
use crate::game::Game;
use crate::glider::Glider;
use crate::level::Level;
use crate::powerups::PowerAssets;
use crate::powerups::Upgrades;
use crate::powerups::pickups::PowerUp;
use crate::powerups::pickups::advance_powerups;
use crate::test_support::pickup;
use crate::web::LANE_UNIT;
use bevy::prelude::*;

#[test]
fn a_drop_forms_for_76_ticks_then_moves_at_the_original_speed() {
    let mut pickup = pickup(100 * LANE_UNIT);
    for _ in 0..76 {
        assert!(!pickup.tick(3, true));
    }
    assert_eq!(pickup.depth, 100 * LANE_UNIT);
    assert_eq!(pickup.rez, -4);
    pickup.tick(3, true);
    assert_eq!(pickup.depth, 100 * LANE_UNIT - 0x18000);
}

#[test]
fn pickup_requires_the_right_lane_range_and_a_grounded_claw() {
    let mut pickup = pickup(32 * LANE_UNIT);
    pickup.rez = -4;
    assert!(!pickup.tick(2, true));
    assert!(!pickup.tick(3, false));
    assert!(pickup.tick(3, true));
    assert_eq!(pickup.radius, 11);
    assert!(pickup.captured);
    let depth = pickup.depth;
    for _ in 0..49 {
        assert!(!pickup.tick(3, true));
    }
    assert_eq!(
        pickup.depth, depth,
        "the pickup burst stays at its collection point"
    );
    assert!(pickup.radius >= 255);
    let mut passed = crate::test_support::pickup(-LANE_UNIT);
    passed.rez = -4;
    assert!(!passed.tick(3, true), "a passed pickup cannot be collected");
}

#[test]
fn collection_grants_one_reward_and_the_burst_expires() {
    let mut app = App::new();
    app.init_resource::<Game>()
        .init_resource::<Level>()
        .init_resource::<tankers::TankerAssets>()
        .init_resource::<FlipperAssets>()
        .init_resource::<Upgrades>()
        .init_resource::<PowerAssets>()
        .init_resource::<explosion::BurstAssets>()
        .add_systems(FixedUpdate, advance_powerups);
    app.world_mut().spawn(Glider {
        progress: 3 * LANE_UNIT,
        velocity: 0,
    });
    let mut pickup = pickup(30 * LANE_UNIT);
    pickup.rez = -4;
    let entity = app.world_mut().spawn(pickup).id();
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().resource::<Upgrades>().laser);
    assert_eq!(app.world().resource::<Upgrades>().stage, 1);
    for _ in 0..48 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert!(app.world().get::<PowerUp>(entity).is_some());
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().get::<PowerUp>(entity).is_none());
    assert_eq!(app.world().resource::<Upgrades>().stage, 1);
}
