//! Camera-facing power-up glyphs and dots, with tube rings in web space.

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
        let point = Vec3::new(angle.cos() * radius, angle.sin() * radius, depth);
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
                let ring_distance = distance - (camera.rotation.inverse() * (Vec3::Z * depth)).z;
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
                    dot * (ring_distance.max(0.01) / distance.max(0.01)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{level::Level, web::FRONT_RADIUS};
    use bevy::{camera::CameraProjection, mesh::VertexAttributeValues};

    #[test]
    fn particle_ring_centers_and_cross_sections_use_web_space_under_camera_rotation() {
        for rotation in [
            Quat::IDENTITY,
            Quat::from_euler(EulerRot::YXZ, 0.105, 0.06, 0.),
            Quat::from_rotation_x(0.06),
        ] {
            let camera = Transform::default().with_rotation(rotation);
            for depth in [0., 8. * WORLD_UNIT, 16. * WORLD_UNIT, 24. * WORLD_UNIT] {
                let mut triangles = Vec::new();
                add_ring(
                    &mut triangles,
                    12,
                    0.6,
                    depth,
                    0.37,
                    Color::WHITE,
                    &camera,
                    0.01,
                );
                for dot in triangles.as_chunks::<2>().0 {
                    let center = (dot[0][0].0 + dot[0][2].0) / 2.;
                    assert!(
                        (center.z - depth).abs() < 1e-6,
                        "camera rotation tilted a ring cross section"
                    );
                }
                let center = triangles
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|dot| (dot[0][0].0 + dot[0][2].0) / 2.)
                    .sum::<Vec3>()
                    / 12.;
                assert!(
                    center.distance(Vec3::Z * depth) < 1e-6,
                    "ring depth must not shift sideways"
                );
            }
        }
    }

    #[test]
    fn powerup_tube_projects_onto_its_lane_on_closed_and_flat_webs() {
        for source in [
            include_str!("../../levels/01.toml"),
            include_str!("../../levels/03.toml"),
        ] {
            let level = Level::parse(source, "power-up alignment").unwrap();
            for lane in [0, level.web.start_lane, level.web.lanes() - 1] {
                let mut app = App::new();
                app.init_resource::<Upgrades>()
                    .init_resource::<Assets<Mesh>>()
                    .add_systems(Update, draw_powerups);
                let camera =
                    Transform::from_translation(level.web.camera_position(
                        level.web.glider_transform(level.web.start()).translation,
                    ))
                    .with_rotation(level.web.camera_rotation());
                let projection = PerspectiveProjection {
                    fov: 1.5,
                    aspect_ratio: 1.6,
                    ..default()
                };
                app.world_mut().spawn((
                    Camera3d::default(),
                    camera,
                    Projection::Perspective(projection.clone()),
                ));
                let mut pose = level.web.glider_transform(lane as i32 * LANE_UNIT);
                let start = pose.translation.truncate();
                let entity = app
                    .world_mut()
                    .spawn((
                        PowerUp {
                            lane,
                            depth: 80 * LANE_UNIT,
                            rez: -4,
                            radius: 10,
                            captured: false,
                            mesh: None,
                        },
                        pose,
                        Mesh3d::default(),
                    ))
                    .id();
                app.update();
                pose = *app.world().get::<Transform>(entity).unwrap();
                let handle = &app.world().get::<Mesh3d>(entity).unwrap().0;
                let mesh = app.world().resource::<Assets<Mesh>>().get(handle).unwrap();
                let VertexAttributeValues::Float32x3(vertices) =
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
                else {
                    panic!()
                };
                let project = |p| {
                    projection
                        .get_clip_from_view()
                        .project_point3(camera.compute_affine().inverse().transform_point3(p))
                };
                // The first six vertices are the laser glyph, followed by four 12-dot rings.
                for ring in 0..4 {
                    let ring_vertices = &vertices[6 + ring * 72..6 + (ring + 1) * 72];
                    let center = ring_vertices
                        .as_chunks::<6>()
                        .0
                        .iter()
                        .map(|dot| (Vec3::from_array(dot[0]) + Vec3::from_array(dot[2])) / 2.)
                        .sum::<Vec3>()
                        / 12.;
                    let world = pose.transform_point(center);
                    assert!(world.truncate().distance(start) < 1e-6);
                    let depth = 80. - ring as f32 * 8.;
                    let a = level
                        .web
                        .vertex(lane as f32, FRONT_RADIUS, -depth * WORLD_UNIT);
                    let b = level
                        .web
                        .vertex(lane as f32 + 1., FRONT_RADIUS, -depth * WORLD_UNIT);
                    let lane_point = a.lerp(b, 0.5) + Vec3::Z * 0.06;
                    assert!(
                        project(world).distance(project(lane_point)) < 1e-6,
                        "tube ring left its projected lane"
                    );
                }
            }
        }
    }
}
