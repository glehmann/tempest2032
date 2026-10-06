use std::f32::consts::TAU;

use super::*;

/// `run_prex` dies once the radius index passes 50, so a Flipper's death ring
/// lives for 51 simulation ticks.
const MAX_AGE: u8 = 51;
/// `dpring` starts the two rings at 16 and 14 pixels per ring.
const POINTS: [usize; 2] = [16, 14];
/// `draw_prex` doubles the radius and pulls the second ring 4 object units
/// further up the tube, which is why z shrinks with distance here.
const RING_SCALE: [f32; 2] = [1.0, 2.0];
const RING_DEPTH: [f32; 2] = [0.0, -4.0];
/// Object units per world unit, the same scale `flipper_mesh` draws at.
const WORLD_SCALE: f32 = 0.06;
/// `pring2` plots single pixels, so every dot keeps the same size however far
/// its ring has grown.
const DOT_SIZE: f32 = 0.03;

#[rustfmt::skip]
const SINES: [i8; 256] = [
       0,    3,    6,    9,   12,   15,   18,   21,   24,   27,   30,   33,   36,   39,   42,   45,
      48,   51,   54,   57,   59,   62,   65,   68,   70,   73,   75,   78,   80,   83,   85,   87,
      89,   92,   94,   96,   98,  100,  102,  103,  105,  107,  109,  110,  112,  113,  114,  116,
     117,  118,  119,  120,  121,  122,  123,  123,  124,  125,  125,  126,  126,  126,  126,  126,
     126,  126,  126,  126,  126,  126,  125,  125,  124,  123,  123,  122,  121,  120,  119,  118,
     117,  115,  114,  113,  111,  110,  108,  107,  105,  103,  101,   99,   97,   95,   93,   91,
      89,   87,   85,   82,   80,   77,   75,   72,   70,   67,   64,   62,   59,   56,   53,   51,
      48,   45,   42,   39,   36,   33,   30,   27,   24,   21,   18,   15,   12,    8,    5,    2,
       0,   -3,   -6,   -9,  -12,  -16,  -19,  -22,  -25,  -28,  -31,  -34,  -37,  -40,  -43,  -46,
     -49,  -51,  -54,  -57,  -60,  -63,  -65,  -68,  -71,  -73,  -76,  -78,  -80,  -83,  -85,  -87,
     -90,  -92,  -94,  -96,  -98, -100, -102, -104, -105, -107, -109, -110, -112, -113, -115, -116,
    -117, -118, -119, -120, -121, -122, -123, -124, -124, -125, -125, -126, -126, -126, -126, -126,
    -126, -126, -126, -126, -126, -125, -125, -125, -124, -123, -123, -122, -121, -120, -119, -118,
    -117, -115, -114, -113, -111, -110, -108, -106, -105, -103, -101,  -99,  -97,  -95,  -93,  -91,
     -89,  -86,  -84,  -82,  -79,  -77,  -74,  -72,  -69,  -67,  -64,  -61,  -59,  -56,  -53,  -50,
     -47,  -44,  -41,  -39,  -36,  -33,  -30,  -26,  -23,  -20,  -17,  -14,  -11,   -8,   -5,   -2,
];

/// Jaguar CRY channel lookup, as used by the TOM scanout code. Indexed by
/// `[cyan][red]`.
#[rustfmt::skip]
const RED: [[u8; 16]; 16] = [
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
const GREEN: [[u8; 16]; 16] = [
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
pub(super) fn pulse_hue(counter: u8) -> u8 {
    let cyan = (SINES[counter as usize] as i32 + 0x80) & 0xf0;
    let red = (SINES[(counter.wrapping_add(0x40)) as usize] as i32 + 0x80) >> 4 & 0x0f;
    (cyan | red) as u8
}

fn shade(channel: u8, intensity: u8) -> u8 {
    ((channel as u32 * intensity as u32) >> 8) as u8
}

/// Jaguar CRY pixels carry no blue, so the rings only ever use red and green.
const NO_BLUE: u8 = 0;
/// `pring2` looks like it dims a ring as it grows, taking `255 - radius >> 16` as
/// the intensity. It cannot: by then it has already divided the radius by z, so
/// the shifted byte is always zero and every ring draws at full intensity. The
/// colour sweep is what makes a ring appear to fade.
const FULL_INTENSITY: u8 = 255;

/// The colour of a ring drawn at colour index `radius`. `draw_prex` passes the
/// radius to `pulser` unchanged for the first ring and doubled for the second, so
/// the inner ring only reaches lime while the outer one carries on into orange.
pub(super) fn burst_colour(radius: u8) -> Color {
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
fn ring_triangles(ring: usize, age: u8) -> Vec<[(Vec3, Color); 3]> {
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
fn burst_mesh(age: u8) -> Mesh {
    colored_mesh((0..2).flat_map(|ring| ring_triangles(ring, age)))
}

/// One mesh per frame of the explosion, indexed by age.
#[derive(Resource, Default)]
pub(super) struct BurstAssets {
    frames: Vec<Handle<Mesh>>,
    material: Handle<StandardMaterial>,
}

/// `draw_prex` passes `frames & 0xff` to `pring2` as the ring's starting phase,
/// which spins the dots by 1/256 of a turn every frame.
#[derive(Resource, Default)]
pub(super) struct Frames(u32);

#[derive(Component)]
#[require(LevelActor)]
pub(super) struct Explosion {
    centre: Vec3,
    age: u8,
    mesh: Option<Handle<Mesh>>,
}

/// Spawn the death ring where a Flipper was destroyed.
pub(super) fn spawn_explosion(commands: &mut Commands, assets: &BurstAssets, centre: Vec3) {
    commands.spawn((
        Explosion {
            centre,
            age: 0,
            mesh: None,
        },
        RenderLayers::layer(1),
        Mesh3d(assets.frames.first().cloned().unwrap_or_default()),
        MeshMaterial3d(assets.material.clone()),
        Transform::from_translation(centre),
    ));
}

/// `run_prex` advances the radius once per tick and removes the object once it
/// passes 50.
#[allow(clippy::type_complexity)]
pub(super) fn age_explosions(
    mut commands: Commands,
    assets: Res<BurstAssets>,
    mut frames: ResMut<Frames>,
    mut meshes: ResMut<Assets<Mesh>>,
    cameras: Query<(&Transform, &Projection), (With<Camera3d>, Without<Explosion>)>,
    mut explosions: Query<(Entity, &mut Explosion, &mut Mesh3d, &mut Transform)>,
) {
    frames.0 = frames.0.wrapping_add(1);
    for (entity, mut explosion, mut mesh, mut transform) in &mut explosions {
        explosion.age += 1;
        if explosion.age > MAX_AGE {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(frame) = assets.frames.get(explosion.age as usize - 1)
            && let Some(mut geometry) = meshes.get(frame).cloned()
        {
            if let Some((camera, Projection::Perspective(projection))) = cameras.iter().next() {
                let distance =
                    -(camera.rotation.inverse() * (explosion.centre - camera.translation)).z;
                let dot_size =
                    distance.max(0.01) * (projection.fov * 0.5).tan() * projection.aspect_ratio
                        / 384.0;
                resize_dots(&mut geometry, dot_size / DOT_SIZE);
            }
            if let Some(handle) = &explosion.mesh {
                if let Some(existing) = meshes.get_mut(handle) {
                    *existing = geometry;
                }
            } else {
                explosion.mesh = Some(meshes.add(geometry));
            }
            mesh.0 = explosion.mesh.as_ref().unwrap().clone();
        }
        transform.translation = explosion.centre;
        transform.rotation = Quat::from_rotation_z(frames.0 as f32 * TAU / 256.0);
    }
}

fn resize_dots(mesh: &mut Mesh, scale: f32) {
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

pub(super) fn setup(
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
mod tests {
    use super::*;

    #[test]
    fn distance_compensation_enlarges_dots_without_moving_the_ring() {
        let mut mesh = burst_mesh(25);
        let positions = |mesh: &Mesh| {
            let bevy::mesh::VertexAttributeValues::Float32x3(values) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
            else {
                panic!()
            };
            values.clone()
        };
        let before = positions(&mesh);
        resize_dots(&mut mesh, 4.0);
        let after = positions(&mesh);
        for (before, after) in before
            .as_chunks::<6>()
            .0
            .iter()
            .zip(after.as_chunks::<6>().0)
        {
            let centre = |quad: &[[f32; 3]; 6]| {
                (Vec3::from_array(quad[0]) + Vec3::from_array(quad[2])) * 0.5
            };
            assert!(centre(before).distance(centre(after)) < 1e-5);
            let width = |quad: &[[f32; 3]; 6]| {
                Vec3::from_array(quad[0]).distance(Vec3::from_array(quad[1]))
            };
            assert!((width(after) - width(before) * 4.0).abs() < 1e-5);
        }
    }

    fn rgb(colour: Color) -> [f32; 3] {
        let [r, g, b, _] = LinearRgba::from(colour).to_f32_array();
        [r, g, b]
    }

    #[test]
    fn the_sine_table_matches_the_original() {
        assert_eq!(SINES.len(), 256);
        assert_eq!(SINES[0], 0);
        assert_eq!(SINES[16], 48);
        assert_eq!(SINES[64], 126);
        assert_eq!(SINES[128], 0);
        assert_eq!(SINES[192], -126);
        assert_eq!(*SINES.iter().max().unwrap(), 126);
    }

    #[test]
    fn the_hue_takes_one_nibble_from_each_quarter_turn_of_the_sine_table() {
        for counter in [0u8, 1, 33, 64, 200] {
            let hue = pulse_hue(counter);
            assert_eq!(
                hue >> 4,
                ((SINES[counter as usize] as i32 + 0x80) as u8 & 0xf0) >> 4
            );
            assert_eq!(
                hue & 0x0f,
                ((SINES[counter.wrapping_add(0x40) as usize] as i32 + 0x80) >> 4) as u8 & 0x0f
            );
        }
    }

    #[test]
    fn the_two_rings_sweep_the_sine_table_from_green_into_orange() {
        // draw_prex hands the inner ring its own radius as the colour index and
        // the outer ring twice that.
        let inner = rgb(burst_colour(1));
        let inner_end = rgb(burst_colour(MAX_AGE));
        assert!(
            inner[0] < 0.2 && inner[1] > 0.9 && inner[2] < 0.05,
            "the inner ring starts green {inner:?}"
        );
        assert!(
            inner_end[0] > inner[0] && inner_end[1] > 0.9,
            "the inner ring only reaches lime {inner_end:?}"
        );
        let outer_end = rgb(burst_colour(2 * MAX_AGE));
        assert!(
            outer_end[0] > 0.9 && outer_end[1] < 0.4 && outer_end[2] < 0.05,
            "the outer ring carries on into orange {outer_end:?}"
        );
    }

    #[test]
    fn rings_use_the_original_point_counts_and_growth() {
        const { assert!(RING_DEPTH[1] < 0.0) };
        assert_eq!(POINTS, [16, 14]);
        assert_eq!(RING_SCALE, [1.0, 2.0]);
        assert_eq!(RING_DEPTH[0], 0.0);
        assert_eq!(ring_triangles(0, MAX_AGE).len(), POINTS[0] * 2);
        assert_eq!(ring_triangles(1, MAX_AGE).len(), POINTS[1] * 2);
    }

    #[test]
    fn every_frame_of_the_explosion_is_prebuilt() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<BurstAssets>()
            .init_resource::<Frames>()
            .init_resource::<Time>()
            .add_systems(FixedUpdate, age_explosions);
        let assets = BurstAssets::default();
        let mut commands = app.world_mut().commands();
        spawn_explosion(&mut commands, &assets, Vec3::new(1.0, 2.0, -0.5));
        app.world_mut().flush();
        let live = |app: &mut App| {
            app.world_mut()
                .query::<&Explosion>()
                .iter(app.world())
                .count()
        };
        assert_eq!(live(&mut app), 1);
        for _ in 0..MAX_AGE {
            app.world_mut().run_schedule(FixedUpdate);
        }
        assert_eq!(live(&mut app), 1, "still alive on its last tick");
        app.world_mut().run_schedule(FixedUpdate);
        assert_eq!(live(&mut app), 0);
    }

    #[test]
    fn the_dots_keep_one_size_while_the_rings_grow() {
        // Each frame has its own mesh, which is what stops the dots from
        // stretching with the ring the way the original plots single pixels.
        let dot = |age: u8| {
            let triangle = ring_triangles(0, age)[0];
            (triangle[1].0 - triangle[0].0).length()
        };
        // A dot at angle zero straddles the radius along the x axis, so the midpoint of
        // its x span is exactly the radius it was drawn at.
        let radius = |age: u8| {
            let triangle = ring_triangles(0, age)[0];
            let (low, high) = triangle
                .iter()
                .map(|(point, _)| point.x)
                .fold((f32::MAX, f32::MIN), |(low, high), x| {
                    (low.min(x), high.max(x))
                });
            (low + high) * 0.5
        };
        for age in [1u8, 13, 27, MAX_AGE] {
            assert!(
                (dot(age) - 2.0 * DOT_SIZE).abs() < 1e-5,
                "a dot is {} wide at age {age}",
                dot(age)
            );
            assert!(
                (radius(age) - age as f32 * WORLD_SCALE).abs() < 1e-5,
                "age {age} drew a ring of {}",
                radius(age)
            );
        }
    }
}
