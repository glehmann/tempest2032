//! Pause input and UI stay live while virtual time, fixed updates, and audio stop.

use bevy::{
    app::{RunFixedMainLoop, RunFixedMainLoopSystems},
    prelude::*,
};

#[derive(Resource, Default)]
pub(crate) struct Menu {
    pub(crate) selected: usize,
}

pub(crate) const CHOICES: [&str; 3] = ["RESUME", "RESTART", "EXIT"];

pub(crate) fn running(time: Option<Res<Time<Virtual>>>) -> bool {
    time.is_none_or(|time| !time.is_paused())
}

pub(crate) fn add_systems(app: &mut App) {
    app.init_resource::<Menu>()
        .configure_sets(
            RunFixedMainLoop,
            RunFixedMainLoopSystems::FixedMainLoop
                .run_if(running)
                .run_if(crate::menu::playing)
                .run_if(|game: Res<crate::game::Game>| {
                    !matches!(game.load, Some(crate::level::LoadReason::Restart))
                }),
        )
        .add_systems(
            PreUpdate,
            (input, audio)
                .chain()
                .after(bevy::input::InputSystems)
                .before(crate::powerups::queue_jump)
                .before(crate::powerups::superzapper::queue),
        );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn input(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    gamepads: Query<&Gamepad>,
    mut game: ResMut<crate::game::Game>,
    mut menu: ResMut<Menu>,
    mut time: ResMut<Time<Virtual>>,
    mut upgrades: Option<ResMut<crate::powerups::Upgrades>>,
    mut exit: MessageWriter<AppExit>,
    front: Option<Res<crate::menu::FrontEnd>>,
) {
    if front
        .as_ref()
        .is_some_and(|front| front.screen != crate::menu::Screen::Gameplay)
        || (front.is_some() && game.status == crate::game::Status::GameOver)
    {
        return;
    }
    if game.status == crate::game::Status::GameOver && !time.is_paused() {
        if keys.get_just_pressed().next().is_none()
            && gamepads
                .iter()
                .all(|pad| pad.get_just_pressed().next().is_none())
            && mouse.is_none_or(|mouse| mouse.get_just_pressed().next().is_none())
        {
            return;
        }
        game.load = Some(crate::level::LoadReason::Restart);
    } else if keys.any_just_pressed([KeyCode::KeyP, KeyCode::Pause]) {
        if time.is_paused() {
            time.unpause();
        } else {
            menu.selected = 0;
            time.pause();
        }
    } else if !time.is_paused() {
        return;
    } else if keys.just_pressed(KeyCode::Escape) {
        time.unpause();
    } else if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Space]) {
        match menu.selected {
            0 => time.unpause(),
            1 => {
                game.load = Some(crate::level::LoadReason::Restart);
                time.unpause();
            }
            _ => {
                exit.write(AppExit::Success);
            }
        }
    } else {
        let up = keys.any_just_pressed([KeyCode::ArrowUp, KeyCode::KeyW]);
        let down = keys.any_just_pressed([KeyCode::ArrowDown, KeyCode::KeyS]);
        if up != down {
            menu.selected =
                (menu.selected + if up { CHOICES.len() - 1 } else { 1 }) % CHOICES.len();
        }
    }
    keys.clear();
    if let Some(upgrades) = &mut upgrades {
        upgrades.jump_queued = false;
        upgrades.zapper.queued = false;
    }
}

fn audio(
    time: Res<Time<Virtual>>,
    mut players: Query<(&mut PlaybackSettings, Option<&AudioSink>)>,
) {
    let paused = time.is_paused();
    for (mut settings, sink) in &mut players {
        if settings.paused != paused {
            settings.paused = paused;
        }
        if let Some(sink) = sink {
            if paused && !sink.is_paused() {
                sink.pause();
            } else if !paused && sink.is_paused() {
                sink.play();
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/pause.rs"]
mod tests;
