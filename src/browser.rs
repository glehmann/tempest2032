//! Browser launch options and suspension without changing the fixed simulation.

use bevy::prelude::*;
use clap::Parser;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_namespace = tempestBrowser)]
extern "C" {
    fn active() -> bool;
    fn ready();
    #[wasm_bindgen(js_name = typingInitials)]
    fn typing_initials(active: bool);
}

pub(crate) fn options() -> crate::cli::Cli {
    let search = web_sys::window().unwrap().location().search().unwrap();
    let query = web_sys::UrlSearchParams::new_with_str(&search).unwrap();
    let mut args = vec!["tempest2032".to_string()];
    for name in [
        "level",
        "warp-zone",
        "warp-course",
        "volume",
        "music-volume",
    ] {
        if let Some(value) = query.get(name) {
            args.extend([format!("--{name}"), value]);
        }
    }
    if query.has("fps") {
        args.push("--fps".into());
    }
    crate::cli::Cli::try_parse_from(args)
        .unwrap_or_else(|error| wasm_bindgen::throw_str(&error.to_string()))
}

pub(crate) fn add_systems(app: &mut App) {
    app.add_systems(PostStartup, ready)
        .add_systems(Update, input_context)
        .add_systems(
            PreUpdate,
            suspend
                .after(crate::gamepad::map_controllers)
                .before(crate::menu::input)
                .before(crate::pause::input),
        );
}

fn input_context(front: Res<crate::menu::FrontEnd>, mut previous: Local<Option<bool>>) {
    let typing = front.screen == crate::menu::Screen::Initials;
    if *previous != Some(typing) {
        typing_initials(typing);
        *previous = Some(typing);
    }
}

#[derive(Default)]
struct Suspension {
    inactive: bool,
    was_paused: bool,
}

fn suspend(
    mut state: Local<Suspension>,
    mut time: ResMut<Time<Virtual>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut upgrades: ResMut<crate::powerups::Upgrades>,
) {
    if !active() {
        if !state.inactive {
            state.was_paused = time.is_paused();
            state.inactive = true;
        }
        time.pause();
        keys.reset_all();
        mouse.reset_all();
        upgrades.jump_queued = false;
        upgrades.zapper.queued = false;
    } else if state.inactive {
        state.inactive = false;
        if !state.was_paused {
            time.unpause();
        }
    }
}
