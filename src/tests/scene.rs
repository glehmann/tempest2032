//! Checks that the highlighted lane edges switch with the moving claw.

use crate::glider::Glider;
use crate::glider::move_glider;
use crate::scene::LaneHighlight;
use crate::scene::update_lane;
use crate::test_support::DEPTH;
use crate::test_support::glider_transform;
use crate::test_support::movement_app;
use crate::test_support::rim_vertex;
use crate::test_support::ticks;
use crate::web::BACK_RADIUS;
use crate::web::FRONT_RADIUS;
use crate::web::LANE_UNIT;
use bevy::prelude::*;

#[test]
fn lane_highlights_switch_with_the_claw() {
    let mut app = movement_app(LANE_UNIT / 2 - 512, &[KeyCode::KeyA]);
    app.add_systems(FixedUpdate, update_lane.after(move_glider));
    let edges = [0, 1].map(|edge| {
        app.world_mut()
            .spawn((LaneHighlight(edge), Transform::default()))
            .id()
    });
    ticks(&mut app, 1);
    let glider_position = app
        .world_mut()
        .query_filtered::<&Transform, With<Glider>>()
        .single(app.world())
        .unwrap()
        .translation;
    assert_eq!(glider_position, glider_transform(LANE_UNIT).translation);
    for (edge, entity) in edges.into_iter().enumerate() {
        let expected = (rim_vertex(1.0 + edge as f32, FRONT_RADIUS, 0.0)
            + rim_vertex(1.0 + edge as f32, BACK_RADIUS, -DEPTH))
            * 0.5
            + Vec3::Z * 0.025;
        assert!(
            app.world()
                .get::<Transform>(entity)
                .unwrap()
                .translation
                .distance(expected)
                < 1e-5
        );
    }
}
