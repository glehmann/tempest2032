//! The player glider, drawn as an eight-pose claw on the web rim.
//! Handles lane movement, acceleration, wrapping, and claw animation.

use crate::level::Level;
use crate::mesh::colored_mesh;
use crate::web::LANE_UNIT;
use bevy::prelude::*;

pub(crate) const ACCELERATION: i32 = 0x400;

pub(crate) const MAX_VELOCITY: i32 = 0x6000;

#[derive(Component, Default)]
pub(crate) struct Glider {
    pub(crate) progress: i32,
    pub(crate) velocity: i32,
}

#[derive(Resource)]
pub(crate) struct ClawFrames(pub(crate) [Handle<Mesh>; 8]);

pub(crate) fn claw_mesh(frame: usize) -> Mesh {
    // sclaw0 through sclaw7 share the same six faces in the original obj2d.s.
    let vertices = [
        [
            (8., 3.),
            (1., 4.),
            (3., 6.),
            (4., 13.),
            (1., 15.),
            (8., 14.),
            (8., 16.),
            (14., 11.),
        ],
        [
            (7., 3.),
            (1., 6.),
            (3., 7.),
            (6., 13.),
            (4., 16.),
            (12., 13.),
            (14., 14.),
            (15., 8.),
        ],
        [
            (6., 4.),
            (1., 8.),
            (3., 8.),
            (7., 12.),
            (7., 15.),
            (14., 11.),
            (16., 12.),
            (14., 5.),
        ],
        [
            (6., 4.),
            (1., 9.),
            (3., 9.),
            (8., 12.),
            (8., 15.),
            (15., 10.),
            (17., 10.),
            (13., 4.),
        ],
        [
            (6., 4.),
            (1., 9.),
            (3., 9.),
            (9., 12.),
            (9., 15.),
            (15., 9.),
            (17., 9.),
            (12., 4.),
        ],
        [
            (4., 5.),
            (2., 13.),
            (4., 11.),
            (11., 12.),
            (11., 15.),
            (15., 8.),
            (17., 8.),
            (12., 4.),
        ],
        [
            (3., 8.),
            (4., 14.),
            (6., 13.),
            (12., 13.),
            (13., 16.),
            (15., 7.),
            (17., 6.),
            (11., 3.),
        ],
        [
            (4., 11.),
            (10., 16.),
            (10., 14.),
            (14., 13.),
            (16., 15.),
            (15., 6.),
            (17., 4.),
            (10., 3.),
        ],
    ][frame]
        .map(|(x, y)| Vec3::new((x - 9.0) * 0.075, (y - 9.0) * 0.075, 0.07));
    let colors = [
        Color::srgb(1.0, 0.95, 0.1),
        Color::srgb(1.0, 0.83, 0.02),
        Color::srgb(0.9, 0.69, 0.0),
        Color::srgb(0.68, 0.44, 0.0),
        Color::srgb(0.84, 0.6, 0.0),
        Color::srgb(0.82, 0.62, 0.0),
        Color::srgb(0.92, 0.76, 0.02),
        Color::srgb(1.0, 0.94, 0.1),
    ];
    let mut triangles = Vec::new();
    for face in [
        [0, 1, 2],
        [1, 2, 4],
        [2, 3, 4],
        [3, 5, 4],
        [4, 5, 6],
        [5, 6, 7],
    ] {
        triangles.push(face.map(|i| (vertices[i], colors[i])));
        triangles.push(face.map(|i| (vertices[i] - Vec3::Z * 0.14, Color::srgb(0.35, 0.22, 0.0))));
    }
    let outline = [0, 1, 4, 6, 7, 5, 3, 2, 0];
    for edge in outline.windows(2) {
        let a = vertices[edge[0]];
        let b = vertices[edge[1]];
        let color = Color::srgb(0.5, 0.34, 0.0);
        let quad = [a, b, b - Vec3::Z * 0.14, a - Vec3::Z * 0.14].map(|p| (p, color));
        triangles.extend([[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]]);
    }
    colored_mesh(triangles)
}

pub(crate) fn claw_frame(progress: i32) -> usize {
    // Pose coordinates use the lane tangent, so poses advance with progress.
    ((progress + LANE_UNIT / 2).rem_euclid(LANE_UNIT) >> 13) as usize
}

pub(crate) fn animate_glider(frames: Res<ClawFrames>, mut gliders: Query<(&Glider, &mut Mesh3d)>) {
    for (glider, mut mesh) in &mut gliders {
        let frame = &frames.0[claw_frame(glider.progress)];
        if mesh.0 != *frame {
            mesh.0 = frame.clone();
        }
    }
}

pub(crate) fn move_glider(
    keys: Res<ButtonInput<KeyCode>>,
    level: Res<Level>,
    mut gliders: Query<(&mut Glider, &mut Transform)>,
) {
    let left = keys.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]);
    let right = keys.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]);
    // web5's lane order is reversed in WEB, so its input sign must be reversed too.
    let direction = i32::from(left) - i32::from(right);
    for (mut glider, mut transform) in &mut gliders {
        if direction == 0 || glider.velocity.signum() == -direction {
            glider.velocity = 0;
        } else {
            glider.velocity =
                (glider.velocity + direction * ACCELERATION).clamp(-MAX_VELOCITY, MAX_VELOCITY);
        }
        let progress = glider.progress + glider.velocity;
        glider.progress = level.web.normalize(progress);
        if glider.progress != progress && !level.web.closed {
            glider.velocity = 0;
        }
        let z = transform.translation.z;
        *transform = level.web.glider_transform(glider.progress);
        transform.translation.z = z;
    }
}

#[cfg(test)]
#[path = "tests/glider.rs"]
mod tests;
