//! Camera-facing power-up glyphs and particle rings for drop formation,
//! travel, and the expanding collection animation.

use crate::cry;
use crate::explosion;
use crate::mesh::colored_mesh;
use crate::powerups::Upgrades;
use crate::powerups::pickups::PowerUp;
use crate::web::LANE_UNIT;
use crate::web::WORLD_UNIT;
use bevy::prelude::*;
use std::f32::consts::TAU;

#[allow(clippy::too_many_arguments)]
fn add_ring(
    triangles: &mut Vec<[(Vec3, Color); 3]>,
    count: usize,
    radius: f32,
    depth: f32,
    phase: f32,
    color: Color,
    camera: &Transform,
    dot: f32,
) {
    if radius <= 0.0 {
        return;
    }
    let corner = |point: Vec3, x: f32, y: f32| {
        (
            point + camera.rotation * Vec3::new(x * dot, y * dot, 0.0),
            color,
        )
    };
    for index in 0..count {
        let angle = phase + index as f32 * TAU / count as f32;
        let point = camera.rotation * Vec3::new(angle.cos() * radius, angle.sin() * radius, depth);
        let quad = [
            corner(point, -1.0, -1.0),
            corner(point, 1.0, -1.0),
            corner(point, 1.0, 1.0),
            corner(point, -1.0, 1.0),
        ];
        triangles.extend([[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]]);
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn draw_powerups(
    upgrades: Res<Upgrades>,
    mut meshes: ResMut<Assets<Mesh>>,
    cameras: Query<(&Transform, &Projection), (With<Camera3d>, Without<PowerUp>)>,
    mut pickups: Query<(&mut PowerUp, &mut Mesh3d, &mut Transform)>,
) {
    let Some((camera, Projection::Perspective(projection))) = cameras.iter().next() else {
        return;
    };
    let phase = upgrades.frame as f32 * TAU / 256.0;
    for (mut pickup, mut mesh, mut transform) in &mut pickups {
        transform.translation.z = -(pickup.depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
        transform.rotation = Quat::IDENTITY;
        let distance =
            -(camera.rotation.inverse() * (transform.translation - camera.translation)).z;
        let dot =
            distance.max(0.01) * (projection.fov * 0.5).tan() * projection.aspect_ratio / 384.0;
        let source_depth = (camera.translation.z - transform.translation.z) / WORLD_UNIT;
        let intensity = |depth: f32| (255.0 * (1.0 - depth / 650.0).clamp(0.0, 1.0)) as u8;
        let mut triangles = Vec::new();
        if pickup.rez >= 0 {
            add_ring(
                &mut triangles,
                20,
                pickup.rez as f32 * 0.06,
                0.0,
                phase,
                cry::color(
                    explosion::pulse_hue(upgrades.frame as u8),
                    intensity(source_depth),
                ),
                camera,
                dot,
            );
            for shift in 0..=3 {
                add_ring(
                    &mut triangles,
                    24,
                    ((300 - pickup.rez) >> shift) as f32 * 0.06,
                    0.0,
                    0.0,
                    cry::color(
                        explosion::pulse_hue(pickup.rez as u8),
                        (255 - ((300 - pickup.rez) >> shift)).clamp(0, 255) as u8,
                    ),
                    camera,
                    dot,
                );
            }
        } else {
            if !pickup.captured {
                let vertices = [
                    (7., 9.),
                    (12., 1.),
                    (12., 9.),
                    (13., 9.),
                    (9., 17.),
                    (9., 9.),
                ]
                .map(|(x, y)| camera.rotation * Vec3::new((x - 10.) * 0.06, (9. - y) * 0.06, 0.0));
                for face in [[0, 1, 2], [3, 4, 5]] {
                    triangles.push(std::array::from_fn(|index| {
                        let intensity = (u16::from([128u8, 192, 255][index])
                            * u16::from(intensity(source_depth))
                            / 255) as u8;
                        (vertices[face[index]], cry::color(0x8a, intensity))
                    }));
                }
            }
            for ring in 0..4 {
                let depth = ring as f32 * 8.0 * WORLD_UNIT;
                let hue = (source_depth as i32 - ring * 8) as u8;
                add_ring(
                    &mut triangles,
                    12,
                    pickup.radius as f32 * 0.06,
                    depth,
                    phase,
                    cry::color(
                        explosion::pulse_hue(hue),
                        intensity(source_depth - ring as f32 * 8.0),
                    ),
                    camera,
                    dot * ((distance - depth).max(0.01) / distance.max(0.01)),
                );
            }
        }
        let geometry = colored_mesh(triangles);
        if let Some(handle) = &pickup.mesh {
            if let Some(existing) = meshes.get_mut(handle) {
                *existing = geometry;
            }
        } else {
            pickup.mesh = Some(meshes.add(geometry));
        }
        mesh.0 = pickup.mesh.as_ref().unwrap().clone();
    }
}
