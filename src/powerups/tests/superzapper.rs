use super::*;
use crate::{
    enemies::{aliens, beasts::Beast, mirrors::Mirror, spikers::Spike},
    level::LoadReason,
    test_support::{enemy, ticks},
    web::LANE_UNIT,
};

fn app() -> App {
    let mut app = crate::test_support::app();
    app.init_resource::<Time<Virtual>>()
        .add_systems(PreUpdate, queue.run_if(crate::pause::running))
        .add_systems(FixedUpdate, advance.run_if(crate::pause::running));
    app
}

fn press(app: &mut App) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.reset(KeyCode::KeyZ);
    keys.press(KeyCode::KeyZ);
    app.world_mut().run_schedule(PreUpdate);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
}

fn shot(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            EnemyShot {
                lane: 3,
                depth: 50 * LANE_UNIT,
                angle: 0,
            },
            Transform::default(),
        ))
        .id()
}

#[test]
fn a_short_tap_starts_a_four_tick_sweep_without_spending_the_second_use_while_held() {
    let mut app = app();
    for lane in 0..3 {
        enemy(&mut app, lane, 40 * LANE_UNIT, Motion::Climb);
    }
    shot(&mut app);
    press(&mut app);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyZ);
    ticks(&mut app, 1);
    let count = |app: &mut App| {
        app.world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .count()
            + app
                .world_mut()
                .query::<&EnemyShot>()
                .iter(app.world())
                .count()
    };
    assert_eq!(count(&mut app), 3);
    press(&mut app); // A held press during a sweep cannot queue the second use.
    ticks(&mut app, 3);
    assert_eq!(count(&mut app), 3);
    ticks(&mut app, 13);
    assert_eq!(
        app.world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(
        app.world_mut()
            .query::<&EnemyShot>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(app.world().resource::<Game>().score, 450);
    assert_eq!(app.world().resource::<Upgrades>().zapper.uses, 1);
    assert!(!app.world().resource::<Upgrades>().zapper.active);
    assert_eq!(
        app.world_mut()
            .query::<&crate::powerups::PowerUp>()
            .iter(app.world())
            .count(),
        0
    );
    assert!(app.world_mut().resource_mut::<Upgrades>().drop_due(5));
    assert_eq!(
        app.world()
            .resource::<Game>()
            .announcements
            .iter()
            .filter(|e| e.id == Id::EatElectricDeath)
            .count(),
        1
    );
    app.world_mut().run_schedule(PreUpdate);
    ticks(&mut app, 4);
    assert_eq!(app.world().resource::<Upgrades>().zapper.uses, 1);
}

#[test]
fn a_zap_and_bullet_in_the_same_tick_cannot_score_the_same_enemy_twice() {
    let mut app = app();
    app.add_systems(FixedUpdate, crate::collisions::resolve_hits.after(advance));
    let target = enemy(&mut app, 3, 40 * LANE_UNIT, Motion::Climb);
    let pose = *app.world().get::<Transform>(target).unwrap();
    app.world_mut()
        .spawn((crate::weapons::Bullet { lane: 3 }, pose));
    press(&mut app);
    ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Game>().score, 150);
    assert_eq!(
        app.world_mut()
            .query::<&crate::powerups::PowerUp>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(
        app.world_mut()
            .query::<&crate::weapons::Bullet>()
            .iter(app.world())
            .count(),
        1
    );
}

#[test]
fn second_use_hits_one_enemy_ignores_shots_and_third_use_does_nothing() {
    let mut app = app();
    press(&mut app);
    ticks(&mut app, 1); // Empty first use is still spent.
    let bullet = shot(&mut app);
    for lane in 0..2 {
        enemy(&mut app, lane, 40 * LANE_UNIT, Motion::Climb);
    }
    press(&mut app);
    ticks(&mut app, 20);
    assert!(app.world().get_entity(bullet).is_ok());
    assert_eq!(
        app.world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .count(),
        1
    );
    assert_eq!(app.world().resource::<Game>().score, 150);
    assert_eq!(app.world().resource::<Upgrades>().zapper.uses, 0);
    press(&mut app);
    ticks(&mut app, 8);
    assert_eq!(app.world().resource::<Game>().score, 150);
    assert_eq!(app.world().resource::<Game>().announcements.len(), 1);
}

#[test]
fn sweep_bypasses_beast_armor_and_shot_windows_but_respects_original_immunities() {
    let mut app = app();
    let beast = enemy(&mut app, 2, 80 * LANE_UNIT, Motion::Climb);
    app.world_mut().entity_mut(beast).insert(Beast { stage: 2 });
    let approaching = enemy(&mut app, 3, 200 * LANE_UNIT, Motion::Approach);
    let flipping = enemy(
        &mut app,
        4,
        0,
        Motion::Flip {
            from: 3,
            direction: 1,
            angle: 0,
        },
    );
    let fuse = app
        .world_mut()
        .spawn((
            Fuseball {
                depth: 40 * LANE_UNIT,
                edge: 5,
                motion: fuseballs::Motion::Rail,
                rise: 48,
                tick: 0,
            },
            Transform::default(),
        ))
        .id();
    let mirror = app
        .world_mut()
        .spawn((
            Mirror {
                lane: 6,
                depth: 40 * LANE_UNIT,
                approach: false,
                hits: 5,
                angle: 0,
            },
            Transform::default(),
        ))
        .id();
    let alien = |motion| AlienDroid {
        lane: 7,
        depth: 40 * LANE_UNIT,
        approach: false,
        direction: 1,
        angle: 0,
        motion,
    };
    let climbing = app
        .world_mut()
        .spawn((alien(aliens::Motion::Climb), Transform::default()))
        .id();
    let rim = app
        .world_mut()
        .spawn((alien(aliens::Motion::Ready), Transform::default()))
        .id();
    let spike = app
        .world_mut()
        .spawn((
            Spike {
                lane: 8,
                length: 30 * LANE_UNIT,
            },
            Transform::default(),
        ))
        .id();
    let reflected = shot(&mut app);
    app.world_mut()
        .entity_mut(reflected)
        .insert(Reflected { speed: 0x1c000 });
    let returned = shot(&mut app);
    app.world_mut()
        .entity_mut(returned)
        .insert(ReturnedShot { speed: 0x38000 });
    press(&mut app);
    ticks(&mut app, 40);
    for dead in [beast, flipping, fuse, rim] {
        assert!(app.world().get_entity(dead).is_err());
    }
    for alive in [approaching, mirror, climbing, spike, reflected, returned] {
        assert!(app.world().get_entity(alive).is_ok());
    }
    assert!(app.world().resource::<Game>().score >= 800);
    assert!(!app.world().resource::<Upgrades>().zapper.active);
}

#[test]
fn zapping_a_tanker_releases_its_contents_and_the_sweep_continues_through_them() {
    let mut app = app();
    app.world_mut().spawn((
        Tanker {
            lane: 3,
            depth: 80 * LANE_UNIT,
            approach: false,
        },
        Transform::default(),
    ));
    press(&mut app);
    ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Game>().score, 100);
    assert_eq!(
        app.world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .count(),
        2
    );
    ticks(&mut app, 12);
    assert_eq!(app.world().resource::<Game>().score, 400);
    assert_eq!(
        app.world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .count(),
        0
    );
    assert!(!app.world().resource::<Upgrades>().zapper.active);
}

#[test]
fn automatic_powerup_sweep_preserves_manual_charge_and_restores_a_spent_last_chance() {
    for uses in 0..=2 {
        let mut app = app();
        app.world_mut().resource_mut::<Upgrades>().zapper.uses = uses;
        enemy(&mut app, 3, 40 * LANE_UNIT, Motion::Climb);
        let mut score = 0;
        for _ in 0..6 {
            app.world_mut()
                .resource_mut::<Upgrades>()
                .collect(&mut score);
        }
        ticks(&mut app, 5);
        assert_eq!(app.world().resource::<Game>().score, 150);
        assert_eq!(app.world().resource::<Upgrades>().zapper.uses, uses.max(1));
        assert_eq!(app.world().resource::<Upgrades>().warp_tokens(), 1);
    }
}

#[test]
fn paused_or_nonplaying_input_cannot_spend_or_buffer_a_charge() {
    let mut app = app();
    for status in [
        Status::Entering,
        Status::Captured,
        Status::ShotHit,
        Status::Fried,
        Status::Exiting,
        Status::GameOver,
        Status::Bonus,
    ] {
        app.world_mut().resource_mut::<Game>().status = status;
        press(&mut app);
        assert!(!app.world().resource::<Upgrades>().zapper.queued);
    }
    app.world_mut().resource_mut::<Game>().status = Status::Playing;
    for lane in 0..2 {
        enemy(&mut app, lane, 40 * LANE_UNIT, Motion::Climb);
    }
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    press(&mut app);
    ticks(&mut app, 10);
    assert_eq!(app.world().resource::<Upgrades>().zapper.uses, 2);
    app.world_mut().resource_mut::<Time<Virtual>>().unpause();
    ticks(&mut app, 4);
    assert_eq!(app.world().resource::<Game>().score, 0);
    press(&mut app);
    ticks(&mut app, 1);
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    ticks(&mut app, 100);
    assert_eq!(app.world().resource::<Game>().score, 150);
    app.world_mut().resource_mut::<Time<Virtual>>().unpause();
    ticks(&mut app, 4);
    assert_eq!(app.world().resource::<Game>().score, 300);
}

#[test]
fn reentering_or_restarting_a_web_recharges_and_clears_pending_zaps() {
    for reason in [LoadReason::Retry, LoadReason::Advance, LoadReason::Restart] {
        let mut app = app();
        let mut upgrades = app.world_mut().resource_mut::<Upgrades>();
        upgrades.zapper.uses = 0;
        upgrades.zapper.active = true;
        upgrades.zapper.queued = true;
        upgrades.zapper.bolt = Some(Vec3::ONE);
        app.world_mut().resource_mut::<Game>().load = Some(reason);
        ticks(&mut app, 1);
        let zapper = &app.world().resource::<Upgrades>().zapper;
        assert_eq!(zapper.uses, 2);
        assert!(!zapper.active && !zapper.queued);
        assert!(zapper.bolt.is_none());
    }
}

#[test]
fn lightning_is_finite_vector_geometry_anchored_at_the_claw_and_target() {
    let start = Vec2::new(-100., 50.);
    let end = Vec2::new(40., -20.);
    for frame in [0, 1, u32::MAX] {
        let triangles = bolt_triangles(start, end, frame);
        assert_eq!(triangles.len(), 128);
        assert!(triangles.iter().flatten().all(|(p, _)| p.is_finite()));
        assert!(
            triangles
                .iter()
                .flatten()
                .any(|(p, _)| p.truncate().distance(start) <= 0.21)
        );
        assert!(
            triangles
                .iter()
                .flatten()
                .any(|(p, _)| p.truncate().distance(end) <= 0.21)
        );
    }
    assert_ne!(bolt_triangles(start, end, 0), bolt_triangles(start, end, 1));
}
