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
fn flat_web_uses_half_claw_tracking_and_original_lens_without_sideways_yaw() {
    use crate::camera::fit_camera;
    use crate::web::WORLD_UNIT;
    use bevy::camera::CameraProjection;

    let level =
        crate::level::Level::parse(include_str!("../../levels/03.toml"), "level 3").unwrap();
    let mut app = App::new();
    app.insert_resource(level.clone())
        .add_systems(Update, (follow_glider, fit_camera).chain());
    let player = app
        .world_mut()
        .spawn((
            crate::glider::Glider {
                progress: level.web.start(),
                velocity: 0,
            },
            level.web.glider_transform(level.web.start()),
        ))
        .id();
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            crate::camera::WebCamera,
            Projection::Perspective(PerspectiveProjection {
                aspect_ratio: 1.6,
                ..default()
            }),
            Transform::from_translation(
                level
                    .web
                    .camera_position(level.web.glider_transform(level.web.start()).translation),
            )
            .with_rotation(Quat::from_rotation_y(0.105)),
        ))
        .id();
    app.world_mut().spawn((
        Window {
            resolution: (1920, 1200).into(),
            ..default()
        },
        bevy::window::PrimaryWindow,
    ));
    app.update();
    let pose = *app.world().get::<Transform>(camera).unwrap();
    assert_eq!(
        pose.translation,
        Vec3::new(0., -16. * WORLD_UNIT, 30. * WORLD_UNIT)
    );
    assert!((pose.rotation * Vec3::NEG_Z).x.abs() < 1e-6);
    let Projection::Perspective(projection) = app.world().get::<Projection>(camera).unwrap() else {
        panic!()
    };
    assert!((projection.fov - 2. * (120_f32 / 128.).atan()).abs() < 1e-6);
    let project = |point| {
        projection
            .get_clip_from_view()
            .project_point3(pose.compute_affine().inverse().transform_point3(point))
    };
    let near_left = project(level.web.vertex(0., crate::web::FRONT_RADIUS, 0.));
    let near_right = project(level.web.vertex(11., crate::web::FRONT_RADIUS, 0.));
    let far_left = project(
        level
            .web
            .vertex(0., crate::web::BACK_RADIUS, -level.web.world_depth()),
    );
    let far_right = project(level.web.vertex(
        11.,
        crate::web::BACK_RADIUS,
        -level.web.world_depth(),
    ));
    assert!(
        (near_left.y - near_right.y).abs() < 1e-6,
        "the front edge must stay level"
    );
    assert!((far_left.y - far_right.y).abs() < 1e-6);
    let ratio = (far_left.x - far_right.x).abs() / (near_left.x - near_right.x).abs();
    assert!(
        (0.15..0.16).contains(&ratio),
        "depth foreshortening changed to {ratio}"
    );
    assert!(
        (0.82..0.84).contains(&((1. - near_left.y) * 0.5)),
        "front edge framing"
    );
    assert!(
        (0.57..0.59).contains(&((1. - far_left.y) * 0.5)),
        "far edge framing"
    );
    let target = level.web.glider_transform(8 * LANE_UNIT);
    *app.world_mut().get_mut::<Transform>(player).unwrap() = target;
    for _ in 0..100 {
        app.update();
    }
    let pose = app.world().get::<Transform>(camera).unwrap();
    assert!((pose.translation.x - target.translation.x * 0.5).abs() < 1e-6);
    assert_eq!(pose.rotation, level.web.camera_rotation());
    let next = crate::level::Level::default();
    *app.world_mut().get_mut::<Transform>(player).unwrap() =
        next.web.glider_transform(next.web.start());
    app.insert_resource(next.clone());
    app.update();
    assert_eq!(
        app.world().get::<Transform>(camera).unwrap().rotation,
        next.web.camera_rotation(),
        "leaving the plane restores the normal web camera"
    );
}

#[test]
fn triangle_base_stays_horizontal_under_tracking() {
    use bevy::camera::CameraProjection;
    let level = crate::level::Level::parse(include_str!("../../levels/05.toml"), "5").unwrap();
    let projection = PerspectiveProjection::default().get_clip_from_view();
    for lane in 0..level.web.lanes() {
        let camera = Transform::from_translation(
            level.web.camera_position(
                level
                    .web
                    .glider_transform(lane as i32 * LANE_UNIT)
                    .translation,
            ),
        )
        .with_rotation(level.web.camera_rotation());
        let project =
            |p| projection.project_point3(camera.compute_affine().inverse().transform_point3(p));
        for i in 0..level.web.lanes() {
            let a = level.web.vertex(i as f32, 4., 0.);
            let b = level.web.vertex((i + 1) as f32, 4., 0.);
            if a.y == b.y {
                assert!((project(a).y - project(b).y).abs() < 1e-6);
            }
        }
    }
}

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
