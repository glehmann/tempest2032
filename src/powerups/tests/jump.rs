//! Checks buffered jump taps during firing and landing after fixed-tick jump physics.

use crate::game::Game;
use crate::glider::Glider;
use crate::glider::move_glider;
use crate::level::Level;
use crate::powerups::Upgrades;
use crate::powerups::jump::queue_jump;
use crate::powerups::jump::update_jump;
use crate::weapons::Bullet;
use crate::weapons::ShotAssets;
use crate::weapons::Weapon;
use crate::weapons::fire_bullets;
use bevy::prelude::*;

#[test]
fn a_jump_tap_between_ticks_is_preserved_while_enter_is_held() {
    let mut app = App::new();
    app.insert_resource(Upgrades {
        jump_enabled: true,
        laser: true,
        ..default()
    })
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<Game>()
    .init_resource::<Level>()
    .init_resource::<ShotAssets>()
    .add_systems(PreUpdate, queue_jump)
    .add_systems(
        FixedUpdate,
        (move_glider, update_jump, fire_bullets).chain(),
    );
    app.world_mut()
        .spawn((Glider::default(), Weapon::default(), Transform::default()));
    for key in [KeyCode::KeyJ, KeyCode::ShiftLeft] {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::Enter);
        keys.press(key);
        app.world_mut().run_schedule(PreUpdate);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(key);
        app.world_mut().run_schedule(FixedUpdate);
        assert!(app.world().resource::<Upgrades>().jump_height > 0);
        assert!(
            app.world_mut()
                .query::<&Bullet>()
                .iter(app.world())
                .next()
                .is_some()
        );
        let mut upgrades = app.world_mut().resource_mut::<Upgrades>();
        upgrades.jump_height = 0;
        upgrades.jump_velocity = 0;
    }
}

#[test]
fn jump_lands_and_resets_its_velocity() {
    let mut app = App::new();
    let upgrades = Upgrades {
        jump_enabled: true,
        ..default()
    };
    let mut keys = ButtonInput::default();
    keys.press(KeyCode::KeyJ);
    app.insert_resource(upgrades)
        .insert_resource(keys)
        .init_resource::<Game>()
        .init_resource::<Level>()
        .add_systems(FixedUpdate, update_jump);
    app.world_mut()
        .spawn((Glider::default(), Transform::default()));
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().resource::<Upgrades>().jump_height > 0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyJ);
    for _ in 0..100 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let upgrades = app.world().resource::<Upgrades>();
    assert_eq!(upgrades.jump_height, 0);
    assert_eq!(upgrades.jump_velocity, 0);
}
