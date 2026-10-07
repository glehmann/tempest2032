//! Checks the optional frame-rate label reports the smoothed diagnostic value.

use crate::hud::{FpsLabel, spawn_fps_label, update_fps};
use bevy::diagnostic::{FrameCountPlugin, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::time::{TimePlugin, TimeUpdateStrategy};
use std::time::Duration;

fn fps_app() -> App {
    let mut app = App::new();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_nanos(
        16_666_667,
    )))
    .add_plugins((
        TimePlugin,
        FrameCountPlugin,
        FrameTimeDiagnosticsPlugin::default(),
    ))
    .add_systems(Startup, spawn_fps_label)
    .add_systems(
        Update,
        update_fps.after(FrameTimeDiagnosticsPlugin::diagnostic_system),
    );
    app.update();
    app
}

fn label(app: &mut App) -> String {
    app.world_mut()
        .query_filtered::<&Text, With<FpsLabel>>()
        .single(app.world())
        .unwrap()
        .0
        .clone()
}

#[test]
fn the_label_reports_nothing_before_the_first_frame_and_the_rate_afterwards() {
    let mut app = fps_app();
    assert_eq!(label(&mut app), "");
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(label(&mut app), "60 FPS");
}
