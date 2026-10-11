//! Pause input and UI stay live while virtual time, fixed updates, and audio stop.

use bevy::{
    app::{RunFixedMainLoop, RunFixedMainLoopSystems},
    prelude::*,
};

pub(crate) fn running(time: Option<Res<Time<Virtual>>>) -> bool {
    time.is_none_or(|time| !time.is_paused())
}

pub(crate) fn add_systems(app: &mut App) {
    app.configure_sets(
        RunFixedMainLoop,
        RunFixedMainLoopSystems::FixedMainLoop.run_if(running),
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
