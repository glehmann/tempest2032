//! Pause input and UI stay live while virtual time, fixed updates, and audio stop.

use bevy::{
    app::{RunFixedMainLoop, RunFixedMainLoopSystems},
    prelude::*,
};

#[derive(Component)]
struct Overlay;

pub(crate) fn running(time: Option<Res<Time<Virtual>>>) -> bool {
    time.is_none_or(|time| !time.is_paused())
}

pub(crate) fn add_systems(app: &mut App) {
    app.configure_sets(
        RunFixedMainLoop,
        RunFixedMainLoopSystems::FixedMainLoop.run_if(running),
    )
    .add_systems(Startup, setup)
    .add_systems(
        PreUpdate,
        (input, audio)
            .chain()
            .after(bevy::input::InputSystems)
            .before(crate::powerups::queue_jump),
    )
    .add_systems(Update, draw);
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Overlay,
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::BLACK.with_alpha(0.55)),
        GlobalZIndex(300),
    ));
}

fn input(
    keys: Res<ButtonInput<KeyCode>>,
    mut time: ResMut<Time<Virtual>>,
    mut upgrades: Option<ResMut<crate::powerups::Upgrades>>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        time.unpause();
    } else if keys.any_just_pressed([KeyCode::KeyP, KeyCode::Pause]) {
        if time.is_paused() {
            time.unpause();
        } else {
            time.pause();
        }
    } else {
        return;
    }
    if let Some(upgrades) = &mut upgrades {
        upgrades.jump_queued = false;
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

fn draw(time: Res<Time<Virtual>>, mut overlays: Query<&mut Node, With<Overlay>>) {
    let paused = time.is_paused();
    for mut node in &mut overlays {
        let display = if paused { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
}

#[cfg(test)]
#[path = "tests/pause.rs"]
mod tests;
