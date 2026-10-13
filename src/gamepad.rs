//! Controller input mapped onto the virtual keys the game already reads, so
//! every system keeps a single input path.

use bevy::{
    input::{
        ButtonInput,
        gamepad::{Gamepad, GamepadAxis, GamepadButton},
        keyboard::KeyCode,
    },
    prelude::*,
};

/// Stick travel needed before a direction counts as pressed.
const DEADZONE: f32 = 0.4;

pub(crate) fn add_systems(app: &mut App) {
    app.add_systems(
        PreUpdate,
        map_controllers
            .after(bevy::input::InputSystems)
            .before(crate::pause::input)
            .run_if(not(resource_exists::<crate::capture::MotionCapture>)),
    );
}

fn button_key(button: GamepadButton) -> Option<KeyCode> {
    Some(match button {
        GamepadButton::South => KeyCode::Enter,
        GamepadButton::East => KeyCode::KeyJ,
        GamepadButton::LeftTrigger | GamepadButton::LeftTrigger2 => KeyCode::KeyJ,
        GamepadButton::West => KeyCode::KeyZ,
        GamepadButton::RightTrigger | GamepadButton::RightTrigger2 => KeyCode::KeyZ,
        GamepadButton::Start => KeyCode::KeyP,
        GamepadButton::DPadLeft => KeyCode::ArrowLeft,
        GamepadButton::DPadRight => KeyCode::ArrowRight,
        GamepadButton::DPadUp => KeyCode::ArrowUp,
        GamepadButton::DPadDown => KeyCode::ArrowDown,
        _ => return None,
    })
}

fn axis_key(axis: GamepadAxis, value: f32) -> Option<KeyCode> {
    let (negative, positive) = match axis {
        GamepadAxis::LeftStickX => (KeyCode::ArrowLeft, KeyCode::ArrowRight),
        GamepadAxis::LeftStickY => (KeyCode::ArrowDown, KeyCode::ArrowUp),
        _ => return None,
    };
    match value {
        value if value > DEADZONE => Some(positive),
        value if value < -DEADZONE => Some(negative),
        _ => None,
    }
}

fn keys_of(gamepad: &Gamepad, menu: bool) -> impl Iterator<Item = KeyCode> + '_ {
    gamepad
        .get_pressed()
        .filter_map(move |&button| {
            if menu && button == GamepadButton::East {
                Some(KeyCode::Escape)
            } else {
                button_key(button)
            }
        })
        .chain(
            GamepadAxis::all()
                .into_iter()
                .filter_map(|axis| gamepad.get(axis).and_then(|value| axis_key(axis, value))),
        )
}

pub(crate) fn map_controllers(
    gamepads: Query<&Gamepad>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut held: Local<Vec<KeyCode>>,
    front: Option<Res<crate::menu::FrontEnd>>,
    mut previous_menu: Local<Option<bool>>,
) {
    let menu = front.is_some_and(|front| front.screen != crate::menu::Screen::Gameplay);
    let mut wanted: Vec<KeyCode> = gamepads.iter().flat_map(|pad| keys_of(pad, menu)).collect();
    wanted.sort_unstable();
    wanted.dedup();
    if previous_menu.is_some_and(|previous| previous != menu) {
        for key in &*held {
            keys.release(*key);
        }
        *held = wanted;
        *previous_menu = Some(menu);
        return;
    }
    *previous_menu = Some(menu);
    for key in wanted.iter().filter(|key| !held.contains(key)) {
        keys.press(*key);
    }
    for key in held.iter().filter(|key| !wanted.contains(key)) {
        keys.release(*key);
    }
    *held = wanted;
}

#[cfg(test)]
#[path = "tests/gamepad.rs"]
mod tests;
