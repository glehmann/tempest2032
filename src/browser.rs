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
    #[wasm_bindgen(js_name = capabilities)]
    fn capabilities() -> JsValue;
    #[wasm_bindgen(js_name = touchInput)]
    fn touch_input(context: &str) -> u32;
    #[wasm_bindgen(js_name = canvasWidth)]
    fn canvas_width() -> f32;
}

pub(crate) fn pointer_position(point: Vec2, window: &Window) -> Vec2 {
    // Winit reports CSS pointer positions, while canvas fitting can use backing pixels.
    point * (window.width() / canvas_width().max(1.))
}

pub(crate) fn touch_keys(
    front: &crate::menu::FrontEnd,
    game: &crate::game::Game,
    time: &Time<Virtual>,
) -> impl Iterator<Item = KeyCode> {
    use crate::menu::Screen;
    let context = match front.screen {
        Screen::Welcome => "welcome",
        Screen::Configuration => "configuration",
        Screen::Initials => "initials",
        Screen::Gameplay if game.status == crate::game::Status::GameOver => "gameover",
        Screen::Gameplay if time.is_paused() => "pause",
        Screen::Gameplay => "gameplay",
    };
    let menu = context != "gameplay";
    let mask = touch_input(context);
    [
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        if menu { KeyCode::Enter } else { KeyCode::KeyZ },
        if menu { KeyCode::Escape } else { KeyCode::KeyX },
        KeyCode::KeyC,
        KeyCode::KeyP,
    ]
    .into_iter()
    .enumerate()
    .filter_map(move |(bit, key)| (mask & (1 << bit) != 0).then_some(key))
}

/// Picks the first antialiasing step. Browsers reduce the user agent, so the
/// renderer string and the pointer type are the only host signals available.
pub(crate) fn start_quality() -> crate::antialiasing::Quality {
    use crate::antialiasing::Quality;
    let capabilities = capabilities();
    let field = |name: &str| js_sys::Reflect::get(&capabilities, &name.into()).ok();
    let renderer = field("renderer")
        .and_then(|value| value.as_string())
        .unwrap_or_default()
        .to_lowercase();
    console_log(&format!("Browser renderer: {renderer:?}"));
    let software = ["swiftshader", "llvmpipe", "lavapipe", "software", "warp"]
        .iter()
        .any(|name| renderer.contains(name));
    if software {
        Quality::Off
    } else if field("coarse").and_then(|value| value.as_bool()) == Some(true) {
        Quality::Fxaa
    } else {
        Quality::Full
    }
}

fn console_log(message: &str) {
    web_sys::console::log_1(&message.into());
}

pub(crate) fn options() -> crate::cli::Cli {
    let search = web_sys::window().unwrap().location().search().unwrap();
    let query = web_sys::UrlSearchParams::new_with_str(&search).unwrap();
    let mut args = vec!["tempest2032".to_string()];
    for name in [
        "level",
        "warp-zone",
        "warp-course",
        "warp-gate",
        "volume",
        "music-volume",
    ] {
        if let Some(value) = query.get(name) {
            args.extend([format!("--{name}"), value]);
        }
    }
    if let Some(value) = query.get("aa").filter(|value| value != "auto") {
        args.extend(["--antialiasing".into(), value]);
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
