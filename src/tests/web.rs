//! Checks original untapered web extrusion, lane midpoints, and glider edge orientation.

use crate::test_support::DEPTH;
use crate::test_support::LANES;
use crate::test_support::WEB_PERIOD;
use crate::test_support::glider_transform;
use crate::test_support::rim_vertex;
use crate::web::BACK_RADIUS;
use crate::web::FRONT_RADIUS;
use crate::web::LANE_UNIT;
use bevy::prelude::*;

#[test]
fn claw_stays_at_lane_midpoints_until_the_lane_changes() {
    for edge in 0..LANES {
        for offset in [-LANE_UNIT / 2, -1, 0, LANE_UNIT / 2 - 1] {
            let position = glider_transform(edge as i32 * LANE_UNIT + offset).translation;
            let expected = rim_vertex(edge as f32, FRONT_RADIUS, 0.0)
                .lerp(rim_vertex(edge as f32 + 1.0, FRONT_RADIUS, 0.0), 0.5)
                + Vec3::Z * 0.12;
            assert!(position.distance(expected) < 1e-5);
        }
    }
    assert!(
        glider_transform(0)
            .translation
            .distance(glider_transform(WEB_PERIOD).translation)
            < 1e-5
    );
}

#[test]
fn original_web_is_extruded_without_taper_and_glider_follows_edge_orientation() {
    assert_eq!(
        rim_vertex(0.0, FRONT_RADIUS, 0.0),
        Vec3::new(-2.0 / 3.0, 4.0, 0.0)
    );
    for edge in 0..LANES {
        let near = rim_vertex(edge as f32, FRONT_RADIUS, 0.0);
        let far = rim_vertex(edge as f32, BACK_RADIUS, -DEPTH);
        assert_eq!(near.truncate(), far.truncate());
        let tangent = (rim_vertex(edge as f32 + 1.0, FRONT_RADIUS, 0.0) - near).normalize();
        assert!(
            (glider_transform(edge as i32 * LANE_UNIT).rotation * Vec3::X).distance(tangent) < 1e-5
        );
    }
}
