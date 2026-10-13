use super::*;

fn app(eligible: bool) -> (App, tempfile::TempDir) {
    let directory = tempfile::tempdir().unwrap();
    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<Game>()
        .init_resource::<crate::messages::Messages>()
        .init_resource::<Difficulty>()
        .init_resource::<Levels>()
        .insert_resource(Store::open(Some(directory.path().join("player.toml"))))
        .insert_resource(FrontEnd::new(
            Screen::Welcome,
            eligible,
            vec![crate::level::Level::default()],
        ))
        .add_message::<KeyboardInput>()
        .add_message::<KeyboardFocusLost>()
        .add_message::<Ime>()
        .add_message::<AppExit>()
        .add_systems(Update, input);
    (app, directory)
}

fn key(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(key);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
}

#[test]
fn menus_do_not_compound_difficulty_and_consume_transition_keys() {
    let (mut app, _directory) = app(true);
    key(&mut app, KeyCode::KeyP);
    assert_eq!(
        app.world().resource::<FrontEnd>().screen,
        Screen::Configuration
    );
    for _ in 0..3 {
        key(&mut app, KeyCode::ArrowRight);
    }
    assert_eq!(*app.world().resource::<Difficulty>(), Difficulty::Normal);
    key(&mut app, KeyCode::ArrowLeft);
    assert_eq!(*app.world().resource::<Difficulty>(), Difficulty::Low);
    key(&mut app, KeyCode::Escape);
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.world().resource::<FrontEnd>().screen, Screen::Gameplay);
    assert!(
        !app.world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::Enter)
    );
    let mut expected = crate::level::Level::default();
    expected.scale_difficulty(Difficulty::Low.scale());
    assert!(app.world().resource::<Levels>().0[0] == expected);
    app.world_mut().resource_mut::<FrontEnd>().screen = Screen::Welcome;
    key(&mut app, KeyCode::Enter);
    assert!(app.world().resource::<Levels>().0[0] == expected);
}

#[test]
fn escape_abandons_gameplay_without_saving_or_exiting_but_resumes_when_paused() {
    for eligible in [false, true] {
        let (mut app, _directory) = app(eligible);
        app.insert_resource(Time::<Virtual>::default())
            .init_resource::<crate::pause::Menu>()
            .add_systems(Update, crate::pause::input.after(input));
        app.world_mut().resource_mut::<FrontEnd>().screen = Screen::Gameplay;
        app.world_mut().resource_mut::<Game>().score = 1234;
        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        key(&mut app, KeyCode::Escape);
        assert_eq!(app.world().resource::<FrontEnd>().screen, Screen::Gameplay);
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
        assert_eq!(app.world().resource::<Game>().score, 1234);
        key(&mut app, KeyCode::Escape);
        assert_eq!(app.world().resource::<FrontEnd>().screen, Screen::Welcome);
        assert_eq!(app.world().resource::<Game>().score, 0);
        assert!(app.world().resource::<Game>().load.is_none());
        assert!(
            app.world()
                .resource::<Store>()
                .data
                .tables
                .iter()
                .all(Vec::is_empty)
        );
        assert!(app.world().resource::<Messages<AppExit>>().is_empty());
        app.update();
        assert!(app.world().resource::<Messages<AppExit>>().is_empty());
        key(&mut app, KeyCode::Escape);
        assert!(!app.world().resource::<Messages<AppExit>>().is_empty());
    }
}

#[test]
fn only_eligible_completed_runs_prompt_and_save_exactly_once() {
    for eligible in [false, true] {
        let (mut app, _directory) = app(eligible);
        app.world_mut().resource_mut::<FrontEnd>().screen = Screen::Gameplay;
        let mut game = app.world_mut().resource_mut::<Game>();
        game.status = Status::GameOver;
        game.score = 1234;
        key(&mut app, KeyCode::Enter);
        assert_eq!(
            app.world().resource::<FrontEnd>().screen,
            if eligible {
                Screen::Initials
            } else {
                Screen::Welcome
            }
        );
        assert!(app.world().resource::<Store>().data.tables[1].is_empty());
        if eligible {
            app.world_mut().write_message(Ime::Commit {
                window: Entity::PLACEHOLDER,
                value: "pRz".into(),
            });
            app.update();
            assert_eq!(app.world().resource::<FrontEnd>().initials, *b"PRZ");
            key(&mut app, KeyCode::Enter);
            app.update();
            assert_eq!(app.world().resource::<FrontEnd>().screen, Screen::Welcome);
            let table = &app.world().resource::<Store>().data.tables[1];
            assert_eq!(table.len(), 1);
            assert_eq!(table[0].initials, "PRZ");
            assert_eq!(table[0].score, 1234);
        }
    }
}

#[test]
fn controller_initials_wrap_and_backspace_allows_retyping() {
    let (mut app, _directory) = app(true);
    app.world_mut()
        .resource_mut::<FrontEnd>()
        .enter_initials(42, "AAA");
    key(&mut app, KeyCode::ArrowDown);
    key(&mut app, KeyCode::ArrowRight);
    key(&mut app, KeyCode::ArrowUp);
    assert_eq!(app.world().resource::<FrontEnd>().initials, *b"ZBA");
    app.world_mut()
        .resource_mut::<FrontEnd>()
        .type_letters("xy");
    assert_eq!(app.world().resource::<FrontEnd>().initials, *b"ZXY");
    key(&mut app, KeyCode::Backspace);
    app.world_mut().resource_mut::<FrontEnd>().type_letters("q");
    assert_eq!(app.world().resource::<FrontEnd>().initials, *b"ZXQ");
}

#[test]
fn bundled_definitions_qualify_but_modified_and_extra_levels_do_not() {
    let mut levels = Levels::load(std::path::Path::new("levels")).unwrap();
    assert!(levels.is_standard());
    levels.0[0].flippers.count += 1;
    assert!(!levels.is_standard());
    levels.0[0].flippers.count -= 1;
    levels.0.push(crate::level::Level::default());
    assert!(!levels.is_standard());
}

#[test]
fn held_controller_confirm_does_not_fire_or_submit_the_next_screen() {
    let (mut app, _directory) = app(true);
    app.add_systems(Update, crate::gamepad::map_controllers.before(input));
    let mut pad = Gamepad::default();
    pad.digital_mut().press(GamepadButton::South);
    let entity = app.world_mut().spawn(pad).id();
    app.update();
    assert_eq!(app.world().resource::<FrontEnd>().screen, Screen::Gameplay);
    for _ in 0..5 {
        app.update();
        assert!(
            !app.world()
                .resource::<ButtonInput<KeyCode>>()
                .pressed(KeyCode::Enter)
        );
        assert!(
            !app.world()
                .resource::<ButtonInput<KeyCode>>()
                .pressed(KeyCode::KeyZ)
        );
    }
    app.world_mut().resource_mut::<Store>().data.initials = "XYZ".into();
    app.world_mut()
        .get_mut::<Gamepad>(entity)
        .unwrap()
        .digital_mut()
        .press(GamepadButton::East);
    app.update();
    app.world_mut().resource_mut::<Game>().status = Status::GameOver;
    app.world_mut().resource_mut::<Game>().score = 42;
    app.update();
    assert_eq!(app.world().resource::<FrontEnd>().screen, Screen::Gameplay);
    app.world_mut()
        .get_mut::<Gamepad>(entity)
        .unwrap()
        .digital_mut()
        .release(GamepadButton::South);
    app.update();
    app.world_mut()
        .get_mut::<Gamepad>(entity)
        .unwrap()
        .digital_mut()
        .press(GamepadButton::South);
    app.update();
    assert_eq!(app.world().resource::<FrontEnd>().screen, Screen::Initials);
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(app.world().resource::<FrontEnd>().initials, *b"XYZ");
    assert!(app.world().resource::<Store>().data.tables[1].is_empty());
}

#[test]
fn physical_text_handles_shortcut_letters_and_transition_auto_repeat() {
    let (mut app, _directory) = app(true);
    fn event(app: &mut App, key: KeyCode, value: Option<&str>, state: ButtonState, repeat: bool) {
        if state == ButtonState::Pressed {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
        } else {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .release(key);
        }
        app.world_mut().write_message(KeyboardInput {
            key_code: key,
            logical_key: bevy::input::keyboard::Key::Unidentified(
                bevy::input::keyboard::NativeKey::Unidentified,
            ),
            state,
            text: value.map(Into::into),
            repeat,
            window: Entity::PLACEHOLDER,
        });
        app.update();
    }
    event(&mut app, KeyCode::Enter, None, ButtonState::Pressed, false);
    event(&mut app, KeyCode::Enter, None, ButtonState::Pressed, true);
    assert!(
        !app.world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::Enter)
    );
    event(&mut app, KeyCode::Enter, None, ButtonState::Released, false);
    app.world_mut()
        .resource_mut::<FrontEnd>()
        .enter_initials(42, "AAA");
    for (key, value) in [
        (KeyCode::KeyP, "p"),
        (KeyCode::KeyR, "r"),
        (KeyCode::KeyZ, "z"),
    ] {
        event(&mut app, key, Some(value), ButtonState::Pressed, false);
    }
    assert_eq!(app.world().resource::<FrontEnd>().initials, *b"PRZ");
    assert_eq!(app.world().resource::<FrontEnd>().screen, Screen::Initials);
}

#[test]
fn initials_follow_os_characters_instead_of_physical_key_positions() {
    let (mut app, _directory) = app(true);
    app.world_mut()
        .resource_mut::<FrontEnd>()
        .enter_initials(42, "AAA");
    for (physical, character, text) in [
        (KeyCode::KeyQ, "a", Some("a")),
        (KeyCode::KeyA, "q", None),
        (KeyCode::KeyY, "z", Some("z")),
    ] {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(physical);
        app.world_mut().write_message(KeyboardInput {
            key_code: physical,
            logical_key: Key::Character(character.into()),
            state: ButtonState::Pressed,
            text: text.map(Into::into),
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
    }
    assert_eq!(app.world().resource::<FrontEnd>().initials, *b"AQZ");
    assert_eq!(app.world().resource::<FrontEnd>().screen, Screen::Initials);
    assert!(app.world().resource::<Game>().load.is_none());
}

#[test]
fn pause_exit_returns_to_welcome_without_a_held_confirm_starting_another_run() {
    for controller in [false, true] {
        let (mut app, _directory) = app(true);
        let mut time = Time::<Virtual>::default();
        time.pause();
        app.insert_resource(time)
            .insert_resource(crate::pause::Menu { selected: 2 })
            .add_systems(Update, crate::pause::input.after(input));
        app.world_mut().resource_mut::<FrontEnd>().screen = Screen::Gameplay;
        app.world_mut().resource_mut::<Game>().score = 1234;
        let pad = if controller {
            app.add_systems(Update, crate::gamepad::map_controllers.before(input));
            let mut pad = Gamepad::default();
            pad.digital_mut().press(GamepadButton::South);
            Some(app.world_mut().spawn(pad).id())
        } else {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Enter);
            app.world_mut().write_message(KeyboardInput {
                key_code: KeyCode::Enter,
                logical_key: bevy::input::keyboard::Key::Enter,
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
            None
        };
        app.update();
        assert_eq!(app.world().resource::<FrontEnd>().screen, Screen::Welcome);
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
        assert_eq!(app.world().resource::<Game>().score, 0);
        assert!(app.world().resource::<Game>().load.is_none());
        assert!(
            app.world()
                .resource::<Store>()
                .data
                .tables
                .iter()
                .all(Vec::is_empty)
        );
        for _ in 0..3 {
            if !controller {
                app.world_mut()
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .press(KeyCode::Enter);
            }
            app.update();
            assert_eq!(app.world().resource::<FrontEnd>().screen, Screen::Welcome);
        }
        assert!(app.world().resource::<Messages<AppExit>>().is_empty());
        if let Some(pad) = pad {
            app.world_mut()
                .get_mut::<Gamepad>(pad)
                .unwrap()
                .digital_mut()
                .release(GamepadButton::South);
            app.update();
            app.world_mut()
                .get_mut::<Gamepad>(pad)
                .unwrap()
                .digital_mut()
                .press(GamepadButton::South);
            app.update();
        } else {
            app.world_mut().write_message(KeyboardInput {
                key_code: KeyCode::Enter,
                logical_key: bevy::input::keyboard::Key::Enter,
                state: ButtonState::Released,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
            app.update();
            key(&mut app, KeyCode::Enter);
        }
        assert_eq!(app.world().resource::<FrontEnd>().screen, Screen::Gameplay);
    }
}
