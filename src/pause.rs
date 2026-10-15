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

pub(crate) fn choice_layout(row: usize, viewport: f32) -> (f32, f32) {
    let scale = 0.65 * ((viewport - 24.) / 174.).clamp(0.01, 1.);
    (5. - row as f32 * 25. - 9. * scale, scale)
}

fn choice_at(position: Vec2, size: Vec2) -> Option<usize> {
    let viewport = 240. * size.x / size.y.max(1.);
    let point = (position - size / 2.) * 240. / size.y.max(1.);
    CHOICES.iter().enumerate().find_map(|(row, text)| {
        let (center, scale) = choice_layout(row, viewport);
        let half_width =
            (text.len() as f32 * crate::messages::geometry::ADVANCE / 2. + 16.) * scale;
        (point.x.abs() <= half_width && (point.y + center).abs() <= 12.5).then_some(row)
    })
}

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
                    !matches!(
                        game.load,
                        Some(
                            crate::level::LoadReason::Restart
                                | crate::level::LoadReason::StartLevel { .. }
                        )
                    )
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
    touches: Option<Res<Touches>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    gamepads: Query<&Gamepad>,
    mut game: ResMut<crate::game::Game>,
    mut menu: ResMut<Menu>,
    mut time: ResMut<Time<Virtual>>,
    mut upgrades: Option<ResMut<crate::powerups::Upgrades>>,
    mut front: Option<ResMut<crate::menu::FrontEnd>>,
    mut messages: Option<ResMut<crate::messages::Messages>>,
    mut commands: Commands,
) {
    if front
        .as_ref()
        .is_some_and(|front| front.screen != crate::menu::Screen::Gameplay)
        || (front.is_some() && game.status == crate::game::Status::GameOver)
    {
        return;
    }
    let pointer = windows.iter().next().and_then(|window| {
        let point = touches
            .as_ref()
            .and_then(|touches| {
                touches
                    .iter_just_pressed()
                    .next()
                    .map(|touch| touch.position())
            })
            .or_else(|| {
                mouse
                    .as_ref()
                    .filter(|mouse| mouse.just_pressed(MouseButton::Left))
                    .and_then(|_| window.cursor_position())
            })?;
        #[cfg(target_arch = "wasm32")]
        let point = crate::browser::pointer_position(point, window);
        Rect::from_corners(Vec2::ZERO, window.size())
            .contains(point)
            .then_some((point, window.size()))
    });
    let clicked = time
        .is_paused()
        .then(|| pointer.and_then(|(point, size)| choice_at(point, size)))
        .flatten();
    if let Some(row) = clicked {
        menu.selected = row;
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
    } else if keys.any_just_pressed([KeyCode::KeyP, KeyCode::Pause])
        || (!time.is_paused() && pointer.is_some())
    {
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
    } else if clicked.is_some()
        || keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Space])
    {
        match menu.selected {
            0 => time.unpause(),
            1 => {
                game.load = Some(crate::level::LoadReason::Restart);
                time.unpause();
            }
            _ => {
                if let (Some(front), Some(messages)) =
                    (front.as_deref_mut(), messages.as_deref_mut())
                {
                    crate::menu::abandon(front, &mut game, messages, &mut commands);
                    time.unpause();
                    keys.reset_all();
                }
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
