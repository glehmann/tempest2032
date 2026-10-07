//! Checks that the camera follows a stopped glider and settles without overshooting.

use crate::camera::CAMERA_STEP;
use crate::camera::follow_glider;
use crate::glider::move_glider;
use crate::test_support::START_PROGRESS;
use crate::test_support::camera_position;
use crate::test_support::glider_transform;
use crate::test_support::movement_app;
use crate::test_support::state;
use crate::test_support::ticks;
use crate::web::LANE_UNIT;
use bevy::prelude::*;

#[test]
fn camera_settles_after_the_claw_stops_without_overshooting() {
    let mut app = movement_app(8 * LANE_UNIT, &[]);
    app.add_systems(FixedUpdate, follow_glider.after(move_glider));
    let start = camera_position(glider_transform(START_PROGRESS).translation);
    let target = camera_position(glider_transform(8 * LANE_UNIT).translation);
    let camera = app
        .world_mut()
        .spawn((Camera3d::default(), Transform::from_translation(start)))
        .id();
    ticks(&mut app, 1);
    let first = app.world().get::<Transform>(camera).unwrap().translation;
    assert!(first.distance(target) < start.distance(target));
    assert!((first - start).abs().max_element() <= CAMERA_STEP + 1e-6);
    ticks(&mut app, 100);
    assert_eq!(
        app.world().get::<Transform>(camera).unwrap().translation,
        target
    );
    assert_eq!(state(&mut app), (8 * LANE_UNIT, 0));
}
