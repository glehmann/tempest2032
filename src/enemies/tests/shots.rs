//! Checks Flipper enemy-shot limits, firing delay, and projectile speed.

use crate::enemies::flippers::Motion;
use crate::enemies::shots::EnemyShot;
use crate::enemies::shots::fire_enemy_shots;
use crate::enemies::shots::move_enemy_shots;
use crate::test_support::REAR;
use crate::test_support::app;
use crate::test_support::enemy;
use crate::test_support::ticks;
use bevy::prelude::*;

#[test]
fn flippers_fire_one_enemy_shot_at_a_time_with_original_speed() {
    let mut app = app();
    app.add_systems(FixedUpdate, (move_enemy_shots, fire_enemy_shots).chain());
    enemy(&mut app, 1, REAR, Motion::Climb);
    ticks(&mut app, 112);
    assert!(
        app.world_mut()
            .query::<&EnemyShot>()
            .iter(app.world())
            .next()
            .is_none()
    );
    ticks(&mut app, 1);
    let shot = app
        .world_mut()
        .query_filtered::<Entity, With<EnemyShot>>()
        .single(app.world())
        .unwrap();
    ticks(&mut app, 1);
    assert_eq!(
        app.world().get::<EnemyShot>(shot).unwrap().depth,
        REAR - 0x18000
    );
    assert_eq!(app.world().get::<EnemyShot>(shot).unwrap().angle, 4);
    for _ in 0..100 {
        ticks(&mut app, 1);
        assert!(
            app.world_mut()
                .query::<&EnemyShot>()
                .iter(app.world())
                .count()
                <= 1
        );
    }
}
