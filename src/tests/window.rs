use super::*;
use crate::menu::{FrontEnd, Screen};
use bevy::input::{
    ButtonState,
    keyboard::{Key, KeyboardInput},
};

fn key(app: &mut App, window: Entity, state: ButtonState, repeat: bool) {
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::KeyF,
        logical_key: Key::Character("f".into()),
        state,
        text: Some("f".into()),
        repeat,
        window,
    });
    app.update();
}

fn mode(app: &App, window: Entity) -> WindowMode {
    app.world().get::<Window>(window).unwrap().mode
}

#[test]
fn f_toggles_once_per_keyboard_press_and_remains_available_for_initials() {
    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<Time<Virtual>>()
        .add_message::<KeyboardInput>()
        .insert_resource(FrontEnd::new(Screen::Welcome, false, Vec::new()))
        .add_systems(PreUpdate, toggle_fullscreen);
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    let fullscreen = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyF);
    app.update();
    assert_eq!(mode(&app, window), WindowMode::Windowed);
    key(&mut app, window, ButtonState::Pressed, false);
    assert_eq!(mode(&app, window), fullscreen);
    app.world_mut().resource_mut::<FrontEnd>().screen = Screen::Gameplay;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    key(&mut app, window, ButtonState::Pressed, true);
    key(&mut app, window, ButtonState::Released, false);
    assert_eq!(mode(&app, window), fullscreen);
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    key(&mut app, window, ButtonState::Pressed, false);
    assert_eq!(mode(&app, window), WindowMode::Windowed);
    app.world_mut().resource_mut::<FrontEnd>().screen = Screen::Initials;
    key(&mut app, window, ButtonState::Pressed, false);
    assert_eq!(mode(&app, window), WindowMode::Windowed);
    app.world_mut().resource_mut::<FrontEnd>().screen = Screen::Configuration;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    key(&mut app, window, ButtonState::Pressed, false);
    assert_eq!(mode(&app, window), WindowMode::Windowed);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    key(&mut app, Entity::PLACEHOLDER, ButtonState::Pressed, false);
    assert_eq!(mode(&app, window), WindowMode::Windowed);
    key(&mut app, window, ButtonState::Pressed, false);
    assert_eq!(mode(&app, window), fullscreen);
}
