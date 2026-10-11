use super::*;

#[test]
fn every_action_has_a_controller_binding() {
    for (button, key) in [
        (GamepadButton::South, KeyCode::Enter),
        (GamepadButton::East, KeyCode::KeyJ),
        (GamepadButton::West, KeyCode::KeyZ),
        (GamepadButton::Start, KeyCode::KeyP),
        (GamepadButton::Select, KeyCode::KeyR),
        (GamepadButton::DPadLeft, KeyCode::ArrowLeft),
        (GamepadButton::DPadRight, KeyCode::ArrowRight),
        (GamepadButton::DPadUp, KeyCode::ArrowUp),
        (GamepadButton::DPadDown, KeyCode::ArrowDown),
        (GamepadButton::RightTrigger, KeyCode::Enter),
        (GamepadButton::LeftTrigger2, KeyCode::ShiftLeft),
        (GamepadButton::RightTrigger2, KeyCode::KeyZ),
    ] {
        assert_eq!(button_key(button), Some(key), "{button:?}");
    }
    for button in [
        GamepadButton::North,
        GamepadButton::Mode,
        GamepadButton::LeftThumb,
    ] {
        assert_eq!(button_key(button), None, "{button:?}");
    }
}

#[test]
fn sticks_become_directions_outside_the_deadzone() {
    for (value, expected) in [
        (-1., Some(KeyCode::ArrowLeft)),
        (-DEADZONE, None),
        (0., None),
        (DEADZONE, None),
        (1., Some(KeyCode::ArrowRight)),
    ] {
        assert_eq!(
            axis_key(GamepadAxis::LeftStickX, value),
            expected,
            "x = {value}"
        );
    }
    for (value, expected) in [
        (-1., Some(KeyCode::ArrowDown)),
        (0., None),
        (1., Some(KeyCode::ArrowUp)),
    ] {
        assert_eq!(
            axis_key(GamepadAxis::LeftStickY, value),
            expected,
            "y = {value}"
        );
    }
    assert_eq!(axis_key(GamepadAxis::RightStickX, 1.), None);
}
