//! Checks acceleration, reversal, open-web stops, wrapping, fixed-tick timing, and claw poses.

use crate::glider::ACCELERATION;
use crate::glider::ClawFrames;
use crate::glider::MAX_VELOCITY;
use crate::glider::animate_glider;
use crate::glider::claw_frame;
use crate::glider::claw_mesh;
use crate::glider::move_glider;
use crate::level::Level;
use crate::test_support::LANES;
use crate::test_support::WEB_PERIOD;
use crate::test_support::glider_transform;
use crate::test_support::movement_app;
use crate::test_support::state;
use crate::test_support::ticks;
use crate::web::LANE_UNIT;
use bevy::prelude::*;
use std::time::Duration;

#[test]
fn movement_stops_at_both_open_web_endpoints() {
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/03.toml")),
        "level3",
    )
    .unwrap();
    for (progress, key) in [(0, KeyCode::KeyD), (10 * LANE_UNIT, KeyCode::KeyA)] {
        let mut app = movement_app(progress, &[key]);
        app.insert_resource(level.clone());
        ticks(&mut app, 30);
        assert_eq!(state(&mut app), (progress, 0));
    }
}

#[test]
fn movement_uses_the_active_webs_lane_count() {
    let mut app = movement_app(512, &[KeyCode::ArrowRight]);
    let mut level = app.world_mut().resource_mut::<Level>();
    level.web.vertices.truncate(8);
    level.web.orientations.truncate(8);
    ticks(&mut app, 1);
    assert_eq!(state(&mut app), (8 * LANE_UNIT - 512, -ACCELERATION));
}

#[test]
fn movement_wraps_in_both_directions() {
    let mut right = movement_app(512, &[KeyCode::ArrowRight]);
    ticks(&mut right, 1);
    assert_eq!(state(&mut right), (WEB_PERIOD - 512, -ACCELERATION));

    let mut left = movement_app(WEB_PERIOD - 512, &[KeyCode::KeyA]);
    ticks(&mut left, 1);
    assert_eq!(state(&mut left), (512, ACCELERATION));
}

#[test]
fn right_follows_the_original_web5_from_its_first_lane_to_its_second() {
    // Source lane zero is between (7,2) and (5,3), our lane 15.
    // Its next lane is between (5,3) and (3,5), with midpoint (4,4).
    for key in [KeyCode::ArrowRight, KeyCode::KeyD] {
        let mut app = movement_app(15 * LANE_UNIT, &[key]);
        ticks(&mut app, 8);
        let (progress, _) = state(&mut app);
        let position = glider_transform(progress).translation;
        assert!(position.distance(Vec3::new(-8.0 / 3.0, 8.0 / 3.0, 0.12)) < 1e-5);
    }
    for key in [KeyCode::ArrowLeft, KeyCode::KeyA] {
        let mut app = movement_app(15 * LANE_UNIT, &[key]);
        ticks(&mut app, 8);
        let (progress, _) = state(&mut app);
        assert!(
            glider_transform(progress)
                .translation
                .distance(Vec3::new(0.0, 4.0, 0.12))
                < 1e-5
        );
    }
}

#[test]
fn movement_accelerates_to_original_speed_and_stops_before_reversing() {
    let mut app = movement_app(0, &[KeyCode::KeyA]);
    ticks(&mut app, 1);
    assert_eq!(state(&mut app), (0x400, 0x400));
    ticks(&mut app, 23);
    assert_eq!(state(&mut app), (307_200, MAX_VELOCITY));
    ticks(&mut app, 1);
    assert_eq!(state(&mut app), (331_776, MAX_VELOCITY));

    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release(KeyCode::KeyA);
    keys.press(KeyCode::KeyD);
    ticks(&mut app, 1);
    assert_eq!(state(&mut app), (331_776, 0));
    ticks(&mut app, 1);
    assert_eq!(state(&mut app), (330_752, -ACCELERATION));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyD);
    ticks(&mut app, 10);
    assert_eq!(state(&mut app), (330_752, 0));

    let mut opposed = movement_app(3 * LANE_UNIT, &[KeyCode::ArrowLeft, KeyCode::KeyD]);
    ticks(&mut opposed, 10);
    assert_eq!(state(&mut opposed), (3 * LANE_UNIT, 0));
}

#[test]
fn fixed_simulation_is_independent_of_render_frame_rate() {
    use bevy::time::TimeUpdateStrategy;

    let mut slow = movement_app(0, &[KeyCode::KeyA]);
    let mut fast = movement_app(0, &[KeyCode::KeyA]);
    for (app, nanos) in [(&mut slow, 33_333_334), (&mut fast, 16_666_667)] {
        app.add_plugins(MinimalPlugins)
            .insert_resource(Time::<Fixed>::from_hz(60.0))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_nanos(
                nanos,
            )));
        app.update();
    }
    for _ in 0..30 {
        slow.update();
    }
    for _ in 0..60 {
        fast.update();
    }
    assert_eq!(state(&mut slow), state(&mut fast));
    assert_eq!(state(&mut slow), (143_360, MAX_VELOCITY));
}

#[test]
fn claw_animation_tracks_movement_reversal_and_stopping() {
    let mut app = movement_app(0, &[KeyCode::KeyA]);
    let mut meshes = Assets::<Mesh>::default();
    let frames = ClawFrames(std::array::from_fn(|frame| meshes.add(claw_mesh(frame))));
    let ids = frames.0.clone();
    let glider = app
        .world_mut()
        .query::<Entity>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .entity_mut(glider)
        .insert(Mesh3d(ids[4].clone()));
    app.insert_resource(frames)
        .add_systems(FixedUpdate, animate_glider.after(move_glider));
    let frame = |app: &App| app.world().get::<Mesh3d>(glider).unwrap().0.clone();

    ticks(&mut app, 1);
    assert_eq!(frame(&app), ids[3]);
    ticks(&mut app, 5);
    assert_eq!(frame(&app), ids[1]);
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    input.release(KeyCode::KeyA);
    input.press(KeyCode::KeyD);
    ticks(&mut app, 1);
    assert_eq!(frame(&app), ids[1]);
    ticks(&mut app, 6);
    assert_eq!(frame(&app), ids[4]);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyD);
    ticks(&mut app, 10);
    assert_eq!(frame(&app), ids[4]);
}

#[test]
fn claw_animation_repeats_each_lane_and_across_the_web_seam() {
    for step in 0..8 {
        let phase = -(step as i32 * (LANE_UNIT / 8) + LANE_UNIT / 2);
        for lane in 0..LANES {
            assert_eq!(claw_frame(lane as i32 * LANE_UNIT + phase), step);
        }
        assert_eq!(claw_frame(phase), claw_frame(WEB_PERIOD + phase));
    }
}
