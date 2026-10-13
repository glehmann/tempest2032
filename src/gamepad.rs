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
    let system = map_controllers
        .after(bevy::input::InputSystems)
        .before(crate::pause::input);
    #[cfg(not(target_arch = "wasm32"))]
    let system = system.run_if(not(resource_exists::<crate::capture::MotionCapture>));
    app.add_systems(PreUpdate, system);
}

fn button_key(button: GamepadButton, menu: bool) -> Option<KeyCode> {
    Some(match button {
        GamepadButton::South if menu => KeyCode::Enter,
        GamepadButton::South => KeyCode::KeyZ,
        GamepadButton::East if menu => KeyCode::Escape,
        GamepadButton::East => KeyCode::KeyX,
        GamepadButton::LeftTrigger | GamepadButton::LeftTrigger2 => KeyCode::KeyX,
        GamepadButton::West => KeyCode::KeyC,
        GamepadButton::RightTrigger | GamepadButton::RightTrigger2 => KeyCode::KeyC,
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

fn keys_of<'a>(
    gamepad: &'a Gamepad,
    menu: bool,
    blocked: &'a [GamepadButton],
) -> impl Iterator<Item = KeyCode> + 'a {
    gamepad
        .get_pressed()
        .filter(|button| !blocked.contains(button))
        .filter_map(move |&button| button_key(button, menu))
        .chain(
            GamepadAxis::all()
                .into_iter()
                .filter_map(|axis| gamepad.get(axis).and_then(|value| axis_key(axis, value))),
        )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn map_controllers(
    gamepads: Query<&Gamepad>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut held: Local<Vec<KeyCode>>,
    front: Option<Res<crate::menu::FrontEnd>>,
    mut previous_menu: Local<Option<bool>>,
    game: Option<Res<crate::game::Game>>,
    time: Option<Res<Time<Virtual>>>,
    mut previous_buttons: Local<Vec<GamepadButton>>,
    mut blocked: Local<Vec<GamepadButton>>,
) {
    let menu = front.is_some_and(|front| front.screen != crate::menu::Screen::Gameplay)
        || game.is_some_and(|game| game.status == crate::game::Status::GameOver)
        || time.is_some_and(|time| time.is_paused());
    let buttons: Vec<_> = gamepads
        .iter()
        .flat_map(|pad| pad.get_pressed().copied())
        .collect();
    if let Some(previous) = *previous_menu {
        // Held actions must not become confirm/back when the input context changes.
        for &button in &buttons {
            if previous_buttons.contains(&button)
                && !blocked.contains(&button)
                && button_key(button, previous) != button_key(button, menu)
            {
                blocked.push(button);
            }
        }
    }
    blocked.retain(|button| buttons.contains(button));
    *previous_buttons = buttons;
    *previous_menu = Some(menu);
    let mut wanted: Vec<KeyCode> = gamepads
        .iter()
        .flat_map(|pad| keys_of(pad, menu, &blocked))
        .collect();
    wanted.sort_unstable();
    wanted.dedup();
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
