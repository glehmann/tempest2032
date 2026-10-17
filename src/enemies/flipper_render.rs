//! Flipper facet meshes, level palette updates, and camera-facing approach
//! dots that brighten as the enemies approach the rear of the web.

use crate::enemies::flippers::Flipper;
use crate::enemies::flippers::Motion;
use crate::level::Level;
use crate::mesh::colored_mesh;
use crate::web::WORLD_UNIT;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub(crate) struct FlipperAssets {
    pub(crate) body: Handle<Mesh>,
    pub(crate) point: Handle<Mesh>,
    pub(crate) shaded: Handle<StandardMaterial>,
    pub(crate) point_shades: Vec<Handle<StandardMaterial>>,
    pub(crate) shot: Handle<Mesh>,
}

pub(crate) fn flipper_mesh(color: [u8; 3]) -> Mesh {
    // A Flipper's four tips sit on the corners of its bounding box, so that box
    // decides the whole look. `fverts` uses 16 by 18, which is nearly square, so
    // the X closes towards a plus sign. Sampling both references gives a much
    // shallower cross: the arms meet at roughly 58 degrees, which needs a box
    // about twice as wide as it is tall. So the tips move to (0,0) and (32,16).
    //
    // The inner vertices keep the source's proportion. There they sit halfway
    // between the centre and a tip, which makes each arm about a quarter as
    // wide as it is long. The reference measures the same ratio, so the inner
    // points move out to (8,8) and (24,8) to match.
    //
    // This is a deliberate departure from fverts, made to match how the
    // original actually renders.
    let vertices = [
        (16., 8.),
        (8., 8.),
        (24., 8.),
        (0., 0.),
        (32., 0.),
        (0., 16.),
        (32., 16.),
    ]
    .map(|(x, y)| Vec3::new((x - 16.) * 0.06, (8. - y) * 0.06, 0.0));
    // The original intensities for each face's tip, inner, and centre vertices.
    let faces = [
        ([3, 1, 0], [128u16, 192, 255]),
        ([4, 2, 0], [64, 128, 192]),
        ([5, 1, 0], [128, 192, 255]),
        ([6, 2, 0], [64, 128, 192]),
    ];
    // Measured against the original renderer, a Flipper reads as four flat
    // pure-red facets: no green channel, and no gradient along an arm.
    colored_mesh(faces.map(|(face, intensity)| {
        let red = intensity.iter().copied().max().unwrap();
        let [r, g, b] = color.map(|channel| (red * u16::from(channel) / 255) as u8);
        let color = Color::srgb_u8(r, g, b);
        face.map(|vertex| (vertices[vertex], color))
    }))
}

pub(crate) fn update_enemy_colors(
    level: Res<Level>,
    assets: Res<FlipperAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if !level.is_changed() {
        return;
    }
    if let Some(body) = meshes.get_mut(&assets.body) {
        *body = flipper_mesh(level.appearance.flipper);
    }
}

pub(crate) fn approach_dot_style(
    position: Vec3,
    camera: &Transform,
    projection: &PerspectiveProjection,
) -> (f32, usize) {
    let distance = -(camera.rotation.inverse() * (position - camera.translation)).z;
    let width =
        4.0 * distance.max(0.01) * (projection.fov * 0.5).tan() * projection.aspect_ratio / 384.0;
    // snglpix doubles z_max=1300, in quarter-units, for its depth cue.
    let source_depth = (camera.translation.z - position.z).max(0.0) / WORLD_UNIT;
    let intensity = (255.0 * (1.0 - source_depth / 650.0).clamp(0.0, 1.0)) as usize;
    (width, intensity)
}

/// Shows an approaching enemy as a camera-facing `snglpix` dot.
pub(crate) fn draw_approach_dot(
    pose: &mut Mut<Transform>,
    material: &mut Mut<MeshMaterial3d<StandardMaterial>>,
    shades: &[Handle<StandardMaterial>],
    camera: &Transform,
    projection: &PerspectiveProjection,
) {
    let (width, intensity) = approach_dot_style(pose.translation, camera, projection);
    pose.set_if_neq(Transform {
        rotation: camera.rotation,
        scale: Vec3::splat(width),
        ..**pose
    });
    material.set_if_neq(MeshMaterial3d(shades[intensity].clone()));
}

#[allow(clippy::type_complexity)]
pub(crate) fn draw_approaching_flippers(
    assets: Res<FlipperAssets>,
    cameras: Query<
        (&Transform, &Projection),
        (
            With<Camera3d>,
            Without<Flipper>,
            Without<crate::messages::geometry::OverlayCamera>,
        ),
    >,
    mut enemies: Query<(
        &Flipper,
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let Some((camera, Projection::Perspective(projection))) = cameras.iter().next() else {
        return;
    };
    for (flipper, mut transform, mut material) in &mut enemies {
        if matches!(flipper.motion, Motion::Approach) {
            draw_approach_dot(
                &mut transform,
                &mut material,
                &assets.point_shades,
                camera,
                projection,
            );
        }
    }
}
