//! Enemy-death particle rings with the original sine table, pulsing hues,
//! depth-cued colors, and fixed-tick lifetime.

use crate::level::LevelActor;
use crate::mesh::colored_mesh;
use crate::sines::SINES;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use std::f32::consts::TAU;

/// `run_prex` dies once the radius index passes 50, so a Flipper's death ring
/// lives for 51 simulation ticks.
pub(crate) const MAX_AGE: u8 = 51;

/// `dpring` starts the two rings at 16 and 14 pixels per ring.
pub(crate) const POINTS: [usize; 2] = [16, 14];

/// `draw_prex` doubles the radius and pulls the second ring 4 object units
/// further up the tube, which is why z shrinks with distance here.
pub(crate) const RING_SCALE: [f32; 2] = [1.0, 2.0];

pub(crate) const RING_DEPTH: [f32; 2] = [0.0, -4.0];

/// Object units per world unit, the same scale `flipper_mesh` draws at.
pub(crate) const WORLD_SCALE: f32 = 0.06;

/// `pring2` plots single pixels, so every dot keeps the same size however far
/// its ring has grown.
pub(crate) const DOT_SIZE: f32 = 0.03;

/// Jaguar CRY channel lookup, as used by the TOM scanout code. Indexed by
/// `[cyan][red]`.
#[rustfmt::skip]
pub(crate) const RED: [[u8; 16]; 16] = [
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 34, 19, 0],
    [68, 68, 68, 68, 68, 68, 68, 68, 68, 68, 68, 68, 64, 43, 21, 0],
    [102, 102, 102, 102, 102, 102, 102, 102, 102, 102, 102, 95, 71, 47, 23, 0],
    [135, 135, 135, 135, 135, 135, 135, 135, 135, 135, 130, 104, 78, 52, 26, 0],
    [161, 161, 161, 161, 161, 161, 161, 161, 161, 157, 143, 121, 99, 72, 45, 18],
    [184, 184, 184, 184, 184, 184, 184, 180, 176, 169, 155, 137, 119, 101, 83, 65],
    [204, 204, 204, 204, 200, 196, 188, 180, 172, 164, 152, 140, 128, 116, 104, 92],
    [221, 221, 217, 213, 209, 205, 197, 189, 181, 173, 165, 157, 145, 133, 121, 109],
    [235, 235, 231, 227, 223, 219, 211, 203, 195, 187, 179, 171, 159, 147, 135, 123],
    [246, 246, 242, 238, 234, 230, 222, 214, 206, 198, 190, 182, 170, 158, 146, 134],
    [255, 255, 251, 247, 243, 239, 231, 223, 215, 207, 199, 191, 179, 167, 155, 143],
    [255, 255, 255, 255, 251, 247, 243, 235, 227, 219, 211, 203, 191, 179, 167, 155],
    [255, 255, 255, 255, 255, 251, 247, 239, 231, 223, 215, 207, 199, 191, 179, 167],
    [255, 255, 255, 255, 255, 255, 251, 243, 239, 231, 223, 215, 207, 199, 191, 179],
    [255, 255, 255, 255, 255, 255, 255, 255, 243, 239, 231, 223, 215, 207, 199, 187],
];

#[rustfmt::skip]
pub(crate) const GREEN: [[u8; 16]; 16] = [
    [0, 17, 34, 51, 68, 85, 102, 119, 136, 153, 170, 187, 204, 221, 238, 255],
    [0, 19, 38, 57, 77, 96, 115, 134, 154, 173, 192, 211, 231, 250, 255, 255],
    [0, 21, 43, 64, 86, 107, 129, 150, 172, 193, 215, 236, 255, 255, 255, 255],
    [0, 23, 47, 71, 95, 119, 142, 166, 190, 214, 238, 255, 255, 255, 255, 255],
    [0, 26, 52, 78, 104, 130, 156, 182, 208, 234, 255, 255, 255, 255, 255, 255],
    [0, 28, 56, 84, 112, 140, 168, 196, 224, 252, 255, 255, 255, 255, 255, 255],
    [0, 31, 62, 93, 124, 155, 186, 217, 248, 255, 255, 255, 255, 255, 255, 255],
    [0, 34, 68, 102, 136, 170, 204, 238, 255, 255, 255, 255, 255, 255, 255, 255],
    [0, 37, 74, 111, 148, 185, 222, 255, 255, 255, 255, 255, 255, 255, 255, 255],
    [0, 41, 82, 123, 164, 205, 246, 255, 255, 255, 255, 255, 255, 255, 255, 255],
    [0, 45, 90, 135, 180, 225, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
    [0, 49, 98, 147, 196, 245, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
    [0, 53, 106, 159, 212, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
    [0, 57, 114, 171, 228, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
    [0, 61, 122, 183, 244, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
    [0, 65, 130, 195, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
];

/// `pulser`: the hue's cyan nibble comes from the sine table at the radius, and
/// its red nibble from the table a quarter turn ahead.
pub(crate) fn pulse_hue(counter: u8) -> u8 {
    let cyan = (SINES[counter as usize] as i32 + 0x80) & 0xf0;
    let red = (SINES[(counter.wrapping_add(0x40)) as usize] as i32 + 0x80) >> 4 & 0x0f;
    (cyan | red) as u8
}

pub(crate) fn shade(channel: u8, intensity: u8) -> u8 {
    ((channel as u32 * intensity as u32) >> 8) as u8
}

/// Jaguar CRY pixels carry no blue, so the rings only ever use red and green.
pub(crate) const NO_BLUE: u8 = 0;

/// `pring2` looks like it dims a ring as it grows, taking `255 - radius >> 16` as
/// the intensity. It cannot: by then it has already divided the radius by z, so
/// the shifted byte is always zero and every ring draws at full intensity. The
/// colour sweep is what makes a ring appear to fade.
pub(crate) const FULL_INTENSITY: u8 = 255;

/// The colour of a ring drawn at colour index `radius`. `draw_prex` passes the
/// radius to `pulser` unchanged for the first ring and doubled for the second, so
/// the inner ring only reaches lime while the outer one carries on into orange.
pub(crate) fn burst_colour(radius: u8) -> Color {
    let hue = pulse_hue(radius);
    let cyan = (hue >> 4) as usize;
    let red = (hue & 0x0f) as usize;
    Color::srgb_u8(
        shade(RED[cyan][red], FULL_INTENSITY),
        shade(GREEN[cyan][red], FULL_INTENSITY),
        NO_BLUE,
    )
}

/// The dots of one ring at one age. `draw_prex` hands each ring its own colour
/// index, the outer one doubled, so the two sweep different stretches of the
/// sine table.
pub(crate) fn ring_triangles(ring: usize, age: u8) -> Vec<[(Vec3, Color); 3]> {
    let radius = age as f32 * RING_SCALE[ring] * WORLD_SCALE;
    let colour = burst_colour(age << ring);
    let depth = Vec3::Z * RING_DEPTH[ring] * WORLD_SCALE;
    let corner = |angle: f32, across: f32, along: f32| {
        let radial = Vec3::new(angle.cos(), angle.sin(), 0.0);
        let tangent = Vec3::new(-angle.sin(), angle.cos(), 0.0);
        depth + radial * radius + tangent * across * DOT_SIZE + radial * along * DOT_SIZE
    };
    (0..POINTS[ring])
        .flat_map(|index| {
            let angle = index as f32 * TAU / POINTS[ring] as f32;
            let quad = [
                corner(angle, -1.0, -1.0),
                corner(angle, 1.0, -1.0),
                corner(angle, 1.0, 1.0),
                corner(angle, -1.0, 1.0),
            ];
            [[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]]
        })
        .map(|triangle| triangle.map(|point| (point, colour)))
        .collect()
}

/// Both rings of a single frame, already coloured and sized in world units.
pub(crate) fn burst_mesh(age: u8) -> Mesh {
    colored_mesh((0..2).flat_map(|ring| ring_triangles(ring, age)))
}

/// One mesh per frame of the explosion, indexed by age.
#[derive(Resource, Default)]
pub(crate) struct BurstAssets {
    pub(crate) frames: Vec<Handle<Mesh>>,
    pub(crate) material: Handle<StandardMaterial>,
}

/// `draw_prex` passes `frames & 0xff` to `pring2` as the ring's starting phase,
/// which spins the dots by 1/256 of a turn every frame.
#[derive(Resource, Default)]
pub(crate) struct Frames(pub(crate) u32);

#[derive(Component)]
#[require(LevelActor)]
pub(crate) struct Explosion {
    pub(crate) centre: Vec3,
    pub(crate) age: u8,
    pub(crate) mesh: Option<Handle<Mesh>>,
}

#[derive(Component)]
pub(crate) struct SphereBurst;

pub(crate) fn spawn_sphere(commands: &mut Commands, assets: &BurstAssets, centre: Vec3) {
    let entity = spawn_explosion(commands, assets, centre);
    commands.entity(entity).insert(SphereBurst);
}

/// Spawn the death ring where a Flipper was destroyed.
pub(crate) fn spawn_explosion(
    commands: &mut Commands,
    assets: &BurstAssets,
    centre: Vec3,
) -> Entity {
    crate::sound::play(commands, crate::sound::Effect::Explosion);
    commands
        .spawn((
            Explosion {
                centre,
                age: 0,
                mesh: None,
            },
            RenderLayers::layer(1),
            Mesh3d(assets.frames.first().cloned().unwrap_or_default()),
            MeshMaterial3d(assets.material.clone()),
            Transform::from_translation(centre),
        ))
        .id()
}

fn sphere_mesh(
    age: u8,
    frame: u32,
    centre: Vec3,
    camera: &Transform,
    projection: &PerspectiveProjection,
) -> Mesh {
    let radius = (1. + 2. * f32::from(age)) * crate::web::WORLD_UNIT;
    let hue =
        ((centre.x.to_bits() ^ centre.y.to_bits()) as u8 & 0xf0) | (15 - (1 + 2 * age) / 4) & 15;
    colored_mesh((0..16).flat_map(|ring| {
        (0..32).flat_map(move |point| {
            let a = point as f32 * TAU / 32.;
            let b = (frame as f32 + ring as f32 * 8.) * TAU / 256.;
            let offset = Vec3::new(a.sin() * b.cos(), -a.cos(), a.sin() * b.sin()) * radius;
            let distance = -(camera.rotation.inverse() * (centre + offset - camera.translation)).z;
            let dot =
                distance.max(0.01) * (projection.fov * 0.5).tan() * projection.aspect_ratio / 384.;
            let color = crate::cry::color(
                hue,
                ((255. - distance / crate::web::WORLD_UNIT / 64.).max(0.)
                    * (255. - (1. + 2. * f32::from(age)))
                    / 256.) as u8,
            );
            let quad = [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)].map(|(x, y)| {
                (
                    offset + camera.rotation * Vec3::new(x * dot, y * dot, 0.),
                    color,
                )
            });
            [[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]]
        })
    }))
}

/// `run_prex` advances the radius once per tick and removes the object once it
/// passes 50.
#[allow(clippy::type_complexity)]
pub(crate) fn age_explosions(
    mut commands: Commands,
    assets: Res<BurstAssets>,
    mut frames: ResMut<Frames>,
    mut meshes: ResMut<Assets<Mesh>>,
    cameras: Query<
        (&Transform, &Projection),
        (
            With<Camera3d>,
            Without<Explosion>,
            Without<crate::messages::geometry::OverlayCamera>,
        ),
    >,
    mut explosions: Query<(
        Entity,
        &mut Explosion,
        &mut Mesh3d,
        &mut Transform,
        Option<&SphereBurst>,
    )>,
) {
    frames.0 = frames.0.wrapping_add(1);
    for (entity, mut explosion, mut mesh, mut transform, sphere) in &mut explosions {
        explosion.age += 1;
        if explosion.age > if sphere.is_some() { 31 } else { MAX_AGE } {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(frame) = assets.frames.get(explosion.age as usize - 1) {
            let geometry = if let (Some(_), Some((camera, Projection::Perspective(projection)))) =
                (sphere, cameras.iter().next())
            {
                sphere_mesh(
                    explosion.age,
                    frames.0,
                    explosion.centre,
                    camera,
                    projection,
                )
            } else {
                let Some(mut geometry) = meshes.get(frame).cloned() else {
                    continue;
                };
                if let Some((camera, Projection::Perspective(projection))) = cameras.iter().next() {
                    let distance =
                        -(camera.rotation.inverse() * (explosion.centre - camera.translation)).z;
                    let dot_size =
                        distance.max(0.01) * (projection.fov * 0.5).tan() * projection.aspect_ratio
                            / 384.0;
                    resize_dots(&mut geometry, dot_size / DOT_SIZE);
                }
                geometry
            };
            if let Some(handle) = &explosion.mesh {
                if let Some(existing) = meshes.get_mut(handle) {
                    *existing = geometry;
                }
            } else {
                let handle = meshes.add(geometry);
                mesh.0 = handle.clone();
                explosion.mesh = Some(handle);
            }
        }
        transform.set_if_neq(Transform {
            translation: explosion.centre,
            rotation: if sphere.is_some() {
                Quat::IDENTITY
            } else {
                Quat::from_rotation_z(frames.0 as f32 * TAU / 256.0)
            },
            ..*transform
        });
    }
}

pub(crate) fn resize_dots(mesh: &mut Mesh, scale: f32) {
    if let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for quad in positions.as_chunks_mut::<6>().0 {
            let centre = (Vec3::from_array(quad[0]) + Vec3::from_array(quad[2])) * 0.5;
            for position in quad {
                *position = (centre + (Vec3::from_array(*position) - centre) * scale).to_array();
            }
        }
    }
}

pub(crate) fn setup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.init_resource::<Frames>();
    let frames: Vec<Handle<Mesh>> = (1..=MAX_AGE)
        .map(|age| meshes.add(burst_mesh(age)))
        .collect();
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    commands.insert_resource(BurstAssets { frames, material });
}

#[cfg(test)]
#[path = "tests/explosion.rs"]
mod tests;
