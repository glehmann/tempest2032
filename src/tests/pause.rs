//! Pause input, full fixed-loop suspension, virtual clocks, and resume UI.

use super::*;
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[derive(Resource, Default)]
struct Ticks(u32);

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(Time::<Fixed>::from_hz(60.))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_nanos(
            16_666_667,
        )))
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::powerups::Upgrades>()
        .init_resource::<Ticks>()
        .add_systems(FixedUpdate, |mut ticks: ResMut<Ticks>| ticks.0 += 1);
    add_systems(&mut app);
    app.update();
    app
}

fn key(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset(key);
}

#[test]
fn pause_stops_fixed_updates_and_resumes_without_catching_up() {
    let mut app = app();
    for _ in 0..10 {
        app.update();
    }
    let ticks = app.world().resource::<Ticks>().0;
    key(&mut app, KeyCode::KeyP);
    assert_eq!(app.world().resource::<Ticks>().0, ticks);
    let elapsed = app.world().resource::<Time<Virtual>>().elapsed();
    for _ in 0..600 {
        app.update();
    }
    assert_eq!(app.world().resource::<Ticks>().0, ticks);
    assert_eq!(app.world().resource::<Time<Virtual>>().elapsed(), elapsed);
    key(&mut app, KeyCode::Pause);
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(app.world().resource::<Ticks>().0, ticks + 10);
}

#[test]
fn keyboard_pause_and_resume_update_visibility_without_text_or_buttons() {
    let mut app = app();
    key(&mut app, KeyCode::KeyP);
    assert!(app.world().resource::<Time<Virtual>>().is_paused());
    for _ in 0..10 {
        app.update();
    }
    assert!(app.world().resource::<Time<Virtual>>().is_paused());
    assert_eq!(
        app.world_mut()
            .query_filtered::<&Node, With<Overlay>>()
            .single(app.world())
            .unwrap()
            .display,
        Display::Flex
    );
    assert_eq!(
        app.world_mut().query::<&Text>().iter(app.world()).count(),
        0
    );
    assert_eq!(
        app.world_mut().query::<&Button>().iter(app.world()).count(),
        0
    );
    key(&mut app, KeyCode::Pause);
    assert!(!app.world().resource::<Time<Virtual>>().is_paused());
    assert_eq!(
        app.world_mut()
            .query_filtered::<&Node, With<Overlay>>()
            .single(app.world())
            .unwrap()
            .display,
        Display::None
    );
}

#[test]
fn restart_resumes_and_discarded_jump_inputs_do_not_reappear() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<crate::powerups::Upgrades>()
        .jump_queued = true;
    key(&mut app, KeyCode::KeyP);
    assert!(
        !app.world()
            .resource::<crate::powerups::Upgrades>()
            .jump_queued
    );
    key(&mut app, KeyCode::KeyR);
    assert!(!app.world().resource::<Time<Virtual>>().is_paused());
}

#[test]
fn pending_audio_is_suspended_without_replacing_its_settings() {
    let mut app = app();
    let audio = app
        .world_mut()
        .spawn(PlaybackSettings::ONCE.with_speed(1.5))
        .id();
    key(&mut app, KeyCode::KeyP);
    let settings = app.world().get::<PlaybackSettings>(audio).unwrap();
    assert!(settings.paused);
    assert_eq!(settings.speed, 1.5);
    key(&mut app, KeyCode::KeyP);
    assert!(!app.world().get::<PlaybackSettings>(audio).unwrap().paused);
}
