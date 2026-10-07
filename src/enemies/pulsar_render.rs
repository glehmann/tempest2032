//! The six original yellow Pulsar zigzag meshes, synchronized pulse poses,
//! camera-facing approach dots, and flashing electrified web segments.

use bevy::prelude::*;

use crate::{
    cry,
    enemies::{
        flipper_render::approach_dot_style,
        pulsars::{Pulsar, Pulse},
    },
    level::Level,
    mesh::colored_mesh,
    scene::{WebAssets, WebSegment},
};

const CYCLE: [usize; 16] = [0, 0, 0, 1, 2, 3, 4, 5, 5, 4, 3, 2, 1, 0, 0, 0];

#[derive(Resource, Default)]
pub(crate) struct PulsarAssets {
    pub(crate) frames: [Handle<Mesh>; 6],
    pub(crate) point: Handle<Mesh>,
    pub(crate) material: Handle<StandardMaterial>,
    pub(crate) shades: Vec<Handle<StandardMaterial>>,
}

fn mesh(frame: usize) -> Mesh {
    // obj2d.s spv6..spv1 and their six triangle faces.
    let h = frame as f32 + 1.0;
    let vertices = [
        (0., 9.),
        (3., 9. + h),
        (3., 7. + h),
        (6., 9.),
        (9., 9. - h),
        (9., 11. - h),
        (12., 9.),
        (15., 9. + h),
        (15., 7. + h),
        (18., 9.),
    ]
    .map(|(x, y)| Vec3::new((x - 9.) * 0.075, (9. - y) * 0.075, 0.));
    let (outer, inner) = [
        (0xf5, 0xfa),
        (0xf7, 0xfb),
        (0xf9, 0xfc),
        (0xfb, 0xfd),
        (0xfd, 0xfe),
        (0xff, 0xff),
    ][frame];
    let intensity = match frame {
        5 => [255; 10],
        4 => [128, 192, 192, 255, 255, 255, 255, 192, 192, 128],
        3 => [96, 160, 160, 224, 255, 255, 224, 160, 160, 96],
        _ => [64, 128, 128, 192, 255, 255, 192, 128, 128, 64],
    };
    colored_mesh(
        [
            [0, 1, 2],
            [1, 2, 3],
            [3, 4, 5],
            [4, 5, 6],
            [6, 7, 8],
            [7, 8, 9],
        ]
        .into_iter()
        .zip([outer, inner, 0xff, 0xff, inner, outer])
        .map(|(face, hue)| face.map(|i| (vertices[i], cry::color(hue, intensity[i])))),
    )
}

pub(crate) fn setup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.insert_resource(PulsarAssets {
        frames: std::array::from_fn(|i| meshes.add(mesh(i))),
        point: meshes.add(Rectangle::new(1., 1.)),
        material: materials.add(StandardMaterial {
            unlit: true,
            cull_mode: None,
            ..default()
        }),
        shades: (0..=255)
            .map(|i| {
                materials.add(StandardMaterial {
                    base_color: cry::color(0xff, i),
                    unlit: true,
                    ..default()
                })
            })
            .collect(),
    });
}

#[allow(clippy::type_complexity)]
pub(crate) fn draw(
    level: Res<Level>,
    pulse: Res<Pulse>,
    assets: Res<PulsarAssets>,
    web: Res<WebAssets>,
    cameras: Query<(&Transform, &Projection), (With<Camera3d>, Without<Pulsar>)>,
    mut pulsars: Query<(
        &Pulsar,
        &mut Transform,
        &mut Mesh3d,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
    mut segments: Query<(&WebSegment, &mut MeshMaterial3d<StandardMaterial>), Without<Pulsar>>,
) {
    let Some((camera, Projection::Perspective(projection))) = cameras.iter().next() else {
        return;
    };
    for (pulsar, mut pose, mut mesh, mut material) in &mut pulsars {
        if pulsar.approach {
            let (width, intensity) = approach_dot_style(pose.translation, camera, projection);
            pose.rotation = camera.rotation;
            pose.scale = Vec3::splat(width);
            mesh.0 = assets.point.clone();
            material.0 = assets.shades[intensity].clone();
        } else {
            pose.scale = Vec3::ONE;
            mesh.0 = assets.frames[CYCLE[pulse.phase as usize]].clone();
            material.0 = assets.material.clone();
        }
    }
    for (segment, mut material) in &mut segments {
        let flashing = pulsars.iter().any(|(p, _, _, _)| {
            if segment.rim {
                p.climbing() && p.lane == segment.lane
            } else {
                p.electrified(pulse.phase)
                    && (p.lane == segment.lane
                        || p.lane + 1 == segment.lane
                        || level.web.closed && segment.lane == 0 && p.lane + 1 == level.web.lanes())
            }
        });
        material.0 = if flashing {
            assets.shades[((pulse.frame & 15) << 4) as usize].clone()
        } else {
            web.blue.clone()
        };
    }
}
