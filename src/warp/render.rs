//! Warp-zone polygons, particle scenes, CRY feedback, and crosshair.

use super::{GATES, Phase, SCALE, ZoneKind};
use crate::{
    cry,
    game::{Game, Status},
    mesh::colored_mesh,
};
use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::RenderLayers,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use std::f32::consts::TAU;

// horse.gas shifts vertex × signed-byte sine by 8, giving 1/2-sized polygons.
const POLYGON_SCALE: f32 = SCALE * 0.5;

#[derive(Component, Clone)]
pub(crate) struct SavedCamera {
    pub(crate) pose: Transform,
    pub(crate) projection: Option<Projection>,
}

#[derive(Component)]
pub(crate) enum WarpNode {
    Ground,
    Gate(usize),
    Icon(usize),
    Victory,
    Tunnel,
    Rings,
}

#[derive(Component)]
pub(crate) struct Crosshair;

#[derive(Resource)]
pub(crate) struct WarpAssets {
    ground: Handle<Mesh>,
    tunnel: Handle<Mesh>,
    rings: Handle<Mesh>,
    feedback: Handle<Image>,
    feedback_material: Handle<StandardMaterial>,
    gates: [Handle<Mesh>; 2],
    icons: [Handle<Mesh>; 5],
}

const SURFACE_WIDTH: u32 = 320;
const SURFACE_HEIGHT: u32 = 128;
const SURFACE_CRY: &[u8; (SURFACE_WIDTH * SURFACE_HEIGHT * 2) as usize] =
    include_bytes!("../../assets/warp/beasty6.cry");

fn surface_image() -> Image {
    let mut pixels = Vec::with_capacity((SURFACE_WIDTH * SURFACE_HEIGHT * 4) as usize);
    for pixel in SURFACE_CRY.as_chunks::<2>().0 {
        let cyan = usize::from(pixel[0] >> 4);
        let red = usize::from(pixel[0] & 15);
        let intensity = u16::from(pixel[1]);
        for table in [&cry::RED, &cry::GREEN, &cry::BLUE] {
            pixels.push(((u16::from(table[cyan][red]) * intensity) >> 8) as u8);
        }
        pixels.push(255);
    }
    let mut image = Image::new(
        Extent3d {
            width: SURFACE_WIDTH,
            height: SURFACE_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        // Mode 7 reflects the texture's vertical offset around its 128-row midpoint.
        address_mode_v: bevy::image::ImageAddressMode::MirrorRepeat,
        mag_filter: bevy::image::ImageFilterMode::Linear,
        min_filter: bevy::image::ImageFilterMode::Linear,
        ..default()
    });
    image
}

fn crosshair_image() -> Image {
    let pixels = include_bytes!("../../assets/warp/crosshair.cry")
        .as_chunks::<2>()
        .0
        .iter()
        .flat_map(|pixel| {
            let color = cry::color(pixel[0], pixel[1]).to_srgba();
            [
                (color.red * 255.) as u8,
                (color.green * 255.) as u8,
                (color.blue * 255.) as u8,
                if *pixel == [0, 0] { 0 } else { 255 },
            ]
        })
        .collect();
    let mut image = Image::new(
        Extent3d {
            width: 40,
            height: 39,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = bevy::image::ImageSampler::nearest();
    image
}

fn gate_mesh(alternate: bool) -> Mesh {
    let vertices = [
        Vec3::new(-17., 0., 0.),
        Vec3::new(-14., 0., 0.),
        Vec3::new(-13., 3., 0.),
        Vec3::new(-13., -3., 0.),
    ];
    colored_mesh((0..8).flat_map(|arm| {
        let rotation = Quat::from_rotation_z(arm as f32 * TAU / 8.);
        [
            ([2, 1, 0], if alternate { 0x62 } else { 0x8c }),
            ([3, 1, 0], if alternate { 0x66 } else { 0x8f }),
        ]
        .map(|(face, hue)| {
            std::array::from_fn(|i| {
                (
                    rotation * vertices[face[i]] * POLYGON_SCALE,
                    cry::color(hue, [192, 255, 64][i]),
                )
            })
        })
    }))
}

#[allow(clippy::type_complexity)]
fn icon_mesh(kind: usize) -> Mesh {
    let (points, faces, hue): (&[(f32, f32)], &[[usize; 3]], u8) = match kind {
        1 | 2 => (
            &[
                (5., 7.),
                (9., 3.),
                (13., 7.),
                (7., 7.),
                (11., 7.),
                (7., 13.),
                (11., 13.),
            ],
            &[[0, 1, 2], [3, 4, 5], [4, 5, 6]],
            0xef,
        ),
        3 => (
            &[
                (9., 5.),
                (5., 9.),
                (7., 9.),
                (11., 9.),
                (13., 9.),
                (9., 9.),
                (5., 13.),
                (7., 13.),
                (11., 13.),
                (13., 13.),
            ],
            &[[0, 1, 2], [0, 3, 4], [5, 6, 7], [5, 8, 9]],
            0x36,
        ),
        _ => (
            &[(9., 13.), (9., 1.), (17., 17.), (1., 17.)],
            &[[1, 2, 0], [2, 3, 0], [3, 1, 0]],
            0xfc,
        ),
    };
    colored_mesh(faces.iter().map(|face| {
        std::array::from_fn(|i| {
            let (x, y) = points[face[i]];
            let point =
                Vec3::new(x - 9., 9. - y, 0.) * POLYGON_SCALE * if kind == 2 { -1. } else { 1. };
            (point, cry::color(hue, [255, 192, 128][i]))
        })
    }))
}

fn ground_mesh(distance: f32, tick: u32) -> Mesh {
    const WIDTH: usize = 32;
    const DEPTH: usize = 48;
    let mut positions = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    for row in 0..=DEPTH {
        let depth = ((row as f32 / DEPTH as f32) * 2000.0_f32.ln()).exp() - 1.;
        let z = -depth * SCALE;
        for column in 0..=WIDTH {
            let x = (column as f32 - 16.) * 16.;
            // Match Mode 7's scanline depth to our 240-line, 1.2-radian projection.
            let mode7_depth = -z * 8. * 0.6_f32.tan() / (120. * SCALE);
            let sine = (mode7_depth * 32. * TAU / 256. + tick as f32 * TAU / 256.).sin();
            let texture_scale = 0.25 - (sine + 1.) * (255. * 4. / 65536.);
            positions.push([x, 0., z]);
            uvs.push([
                0.5 + x * (texture_scale / 0.25) / (SURFACE_WIDTH as f32 * SCALE),
                (distance / SCALE + mode7_depth * texture_scale * (0xff000 as f32 / 65536.))
                    / SURFACE_HEIGHT as f32,
            ]);
            let brightness = (1. - depth / 2000.).max(0.05);
            colors.push([brightness, brightness, brightness, 1.]);
        }
    }
    let mut indices = Vec::new();
    for row in 0..DEPTH {
        for column in 0..WIDTH {
            let a = (row * (WIDTH + 1) + column) as u32;
            let b = a + 1;
            let c = a + (WIDTH + 1) as u32;
            indices.extend([a, b, c, b, c + 1, c]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; positions.len()])
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

fn pixel_mesh(pixels: impl IntoIterator<Item = (Vec2, Color)>, fov: f32) -> Mesh {
    let pixel = 20. * (fov * 0.5).tan() / 240.;
    colored_mesh(
        pixels
            .into_iter()
            .filter(|(p, _)| p.x.abs() <= 192. && p.y.abs() <= 120.)
            .flat_map(|(p, color)| {
                let center = Vec3::new(p.x * pixel, -p.y * pixel, -10.);
                let corners = [
                    Vec3::new(-0.5, -0.5, 0.),
                    Vec3::new(0.5, -0.5, 0.),
                    Vec3::new(0.5, 0.5, 0.),
                    Vec3::new(-0.5, 0.5, 0.),
                ]
                .map(|offset| (center + offset * pixel, color));
                [
                    [corners[0], corners[1], corners[2]],
                    [corners[0], corners[2], corners[3]],
                ]
            }),
    )
}

fn screen_mesh(aspect_ratio: f32) -> Mesh {
    let y = 10. * (1.2_f32 * 0.5).tan();
    let x = y * aspect_ratio;
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[-x, -y, -10.], [x, -y, -10.], [x, y, -10.], [-x, y, -10.]],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 0., 1.]; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0., 1.], [1., 1.], [1., 0.], [0., 0.]],
    )
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
}

pub(crate) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let gates = [meshes.add(gate_mesh(false)), meshes.add(gate_mesh(true))];
    let icons = std::array::from_fn(|kind| meshes.add(icon_mesh(kind)));
    let material = materials.add(StandardMaterial {
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let ground = meshes.add(ground_mesh(0., 0));
    let tunnel = meshes.add(colored_mesh([]));
    let rings = meshes.add(screen_mesh(1.6));
    let mut feedback_image = Image::new_fill(
        Extent3d {
            width: super::rings::WIDTH as u32,
            height: super::rings::HEIGHT as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    feedback_image.sampler = bevy::image::ImageSampler::nearest();
    let feedback = images.add(feedback_image);
    let feedback_material = materials.add(StandardMaterial {
        base_color_texture: Some(feedback.clone()),
        unlit: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        WarpNode::Rings,
        Mesh3d(rings.clone()),
        MeshMaterial3d(feedback_material.clone()),
        Transform::default(),
        Visibility::Hidden,
        RenderLayers::layer(1),
        bevy::camera::visibility::NoFrustumCulling,
    ));
    commands.spawn((
        WarpNode::Tunnel,
        Mesh3d(tunnel.clone()),
        MeshMaterial3d(material.clone()),
        Transform::default(),
        Visibility::Hidden,
        RenderLayers::layer(1),
        bevy::camera::visibility::NoFrustumCulling,
    ));
    let ground_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(surface_image())),
        unlit: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        WarpNode::Ground,
        Mesh3d(ground.clone()),
        MeshMaterial3d(ground_material),
        Transform::default(),
        Visibility::Hidden,
        RenderLayers::layer(0),
    ));
    for index in 0..GATES {
        commands.spawn((
            WarpNode::Gate(index),
            Mesh3d(gates[0].clone()),
            MeshMaterial3d(material.clone()),
            Transform::default(),
            Visibility::Hidden,
            RenderLayers::layer(1),
        ));
        commands.spawn((
            WarpNode::Icon(index),
            Mesh3d(icons[0].clone()),
            MeshMaterial3d(material.clone()),
            Transform::default(),
            Visibility::Hidden,
            RenderLayers::layer(1),
        ));
    }
    commands.spawn((
        WarpNode::Victory,
        Mesh3d(icons[4].clone()),
        MeshMaterial3d(material),
        Transform::default(),
        Visibility::Hidden,
        RenderLayers::layer(1),
    ));
    commands.insert_resource(WarpAssets {
        ground,
        tunnel,
        rings,
        feedback,
        feedback_material,
        gates,
        icons,
    });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(20),
        ))
        .with_children(|parent| {
            parent.spawn((
                Crosshair,
                ImageNode::new(images.add(crosshair_image())),
                Visibility::Hidden,
                Node {
                    height: percent(39. / 240. * 100.),
                    aspect_ratio: Some(40. / 39.),
                    ..default()
                },
            ));
        });
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn draw(
    game: Res<Game>,
    assets: Res<WarpAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut cameras: Query<
        (&mut Transform, &mut Projection),
        (
            With<Camera3d>,
            Without<WarpNode>,
            Without<crate::messages::geometry::OverlayCamera>,
        ),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut nodes: Query<
        (&WarpNode, &mut Transform, &mut Mesh3d, &mut Visibility),
        (Without<Camera3d>, With<WarpNode>),
    >,
    mut crosshairs: Query<&mut Visibility, (With<Crosshair>, Without<Text>, Without<WarpNode>)>,
    mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
) {
    let bonus = game.status == Status::Bonus;
    let session = game.bonus.as_ref().filter(|_| bonus);
    for (_, _, _, mut visible) in &mut nodes {
        *visible = Visibility::Hidden;
    }
    for mut visible in &mut crosshairs {
        *visible = if session
            .is_some_and(|s| s.kind != ZoneKind::Tunnel && matches!(s.phase, Phase::Flying))
        {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let Some(session) = session else { return };
    for mut window in &mut windows {
        window.title = format!(
            "T2K | {} | Course {}",
            session.kind.label(),
            session.course + 1
        );
    }
    let position = if session.kind != ZoneKind::GateFlight {
        Vec2::ZERO
    } else {
        session.position()
    };
    for (mut pose, mut projection) in &mut cameras {
        *pose = Transform::from_xyz(position.x * SCALE, -position.y * SCALE, 0.);
        if let Projection::Perspective(perspective) = &mut *projection {
            perspective.fov = 1.2;
            perspective.near = 0.01;
            perspective.far = 500.;
            if session.rings.is_some()
                && let Some(mesh) = meshes.get_mut(&assets.rings)
            {
                *mesh = screen_mesh(perspective.aspect_ratio);
            }
        }
    }
    if session.kind == ZoneKind::GateFlight
        && let Some(mesh) = meshes.get_mut(&assets.ground)
    {
        *mesh = ground_mesh(session.distance as f32 / 65536. * SCALE, session.tick);
    }
    if let Some(rings) = &session.rings
        && let Some(image) = images.get_mut(&assets.feedback)
    {
        // Updating the texture replaces its GPU view; refresh the material binding too.
        let _ = materials.get_mut(&assets.feedback_material);
        let data = image
            .data
            .get_or_insert_with(|| vec![0; super::rings::WIDTH * super::rings::HEIGHT * 4]);
        for (pixel, rgba) in rings.pixels.iter().zip(data.as_chunks_mut::<4>().0) {
            let hue = (pixel >> 8) as usize;
            for (channel, table) in [&cry::RED, &cry::GREEN, &cry::BLUE].iter().enumerate() {
                rgba[channel] = ((u16::from(table[hue >> 4][hue & 15]) * (pixel & 255)) >> 8) as u8;
            }
            rgba[3] = 255;
        }
    }
    for (node, mut pose, mut mesh, mut visible) in &mut nodes {
        match *node {
            WarpNode::Rings => {
                if session.rings.is_some() {
                    *visible = Visibility::Visible;
                }
            }
            WarpNode::Tunnel => {
                if let Some(tunnel) = &session.tunnel {
                    if let Some(mesh) = meshes.get_mut(&assets.tunnel) {
                        *mesh = pixel_mesh(tunnel.pixels(session.tick), 1.2);
                    }
                    *visible = Visibility::Visible;
                }
            }
            WarpNode::Ground => {
                if session.kind == ZoneKind::GateFlight {
                    *visible = Visibility::Visible;
                }
            }
            WarpNode::Gate(index) | WarpNode::Icon(index) => {
                if session.kind != ZoneKind::GateFlight {
                    continue;
                }
                let Some(gate) = session.gates.iter().find(|g| g.index == index) else {
                    continue;
                };
                let (x, y, kind) = session.spec(index);
                if (y < 0) != (position.y < 0.) {
                    continue;
                }
                *pose = Transform::from_xyz(
                    f32::from(x) * SCALE,
                    -f32::from(y) * SCALE,
                    -gate.distance as f32 / 65536. * SCALE,
                );
                if matches!(*node, WarpNode::Gate(_)) {
                    pose.rotation = Quat::from_rotation_z(-f32::from(gate.age as u8) * TAU / 256.);
                    mesh.0 = assets.gates[usize::from(index % 3 == 2)].clone();
                    *visible = Visibility::Visible;
                } else if kind > 0 {
                    pose.translation.z += 0.01;
                    mesh.0 = assets.icons[usize::from(kind)].clone();
                    *visible = Visibility::Visible;
                }
            }
            WarpNode::Victory => {
                if session.kind != ZoneKind::Tunnel && matches!(session.phase, Phase::Victory(_)) {
                    *pose = Transform::from_xyz(position.x * SCALE, -position.y * SCALE, -4.)
                        .with_rotation(Quat::from_rotation_y(session.tick as f32 * 0.02))
                        .with_scale(Vec3::splat(2.));
                    *visible = Visibility::Visible;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::Level;
    use crate::warp::{Gate, Session, ZoneKind};

    #[test]
    fn rings_and_tunnel_isolate_their_scene_and_upload_visible_feedback() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Game>()
            .init_resource::<Level>()
            .add_systems(Startup, setup)
            .add_systems(Update, draw);
        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(5., 5., 0.),
            Projection::Perspective(PerspectiveProjection::default()),
        ));
        for kind in [ZoneKind::Tunnel, ZoneKind::Rings] {
            let mut session = Session::new(kind, 0, Some(1), 0);
            if let Some(rings) = &mut session.rings {
                for tick in 1..=60 {
                    rings.advance(
                        tick,
                        Vec2::ZERO,
                        &std::collections::VecDeque::from([Gate {
                            index: 0,
                            distance: 200 * 65536,
                            age: 0,
                        }]),
                        &[(0, 16, 0)],
                        Phase::Flying,
                    );
                }
            }
            let mut game = app.world_mut().resource_mut::<Game>();
            game.status = Status::Bonus;
            game.bonus = Some(session);
            app.update();
            let visible: Vec<_> = app
                .world_mut()
                .query::<(&WarpNode, &Visibility)>()
                .iter(app.world())
                .filter(|(_, visibility)| **visibility == Visibility::Visible)
                .map(|(node, _)| node)
                .collect();
            assert_eq!(visible.len(), 1);
            assert!(matches!(
                (kind, visible[0]),
                (ZoneKind::Tunnel, WarpNode::Tunnel) | (ZoneKind::Rings, WarpNode::Rings)
            ));
            if kind == ZoneKind::Rings {
                let assets = app.world().resource::<WarpAssets>();
                let pixels = app
                    .world()
                    .resource::<Assets<Image>>()
                    .get(&assets.feedback)
                    .unwrap()
                    .data
                    .as_ref()
                    .unwrap();
                assert!(
                    pixels
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .filter(|rgba| rgba[..3] != [0, 0, 0])
                        .count()
                        > 128
                );
            }
        }
        app.world_mut().resource_mut::<Game>().status = Status::Playing;
        app.update();
        assert!(
            app.world_mut()
                .query::<(&WarpNode, &Visibility)>()
                .iter(app.world())
                .all(|(_, v)| *v == Visibility::Hidden)
        );
    }

    #[test]
    fn gate_and_icon_sizes_match_the_original_polygon_renderer() {
        use bevy::mesh::VertexAttributeValues;

        for alternate in [false, true] {
            let mesh = gate_mesh(alternate);
            let VertexAttributeValues::Float32x3(positions) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
            else {
                panic!()
            };
            let radii = positions
                .iter()
                .map(|p| Vec3::from_array(*p).length() / SCALE);
            assert!((radii.clone().fold(0., f32::max) - 8.5).abs() < 1e-5);
            assert!((radii.fold(f32::INFINITY, f32::min) - 178_f32.sqrt() / 2.).abs() < 1e-5);
        }
        let mesh = icon_mesh(1);
        let VertexAttributeValues::Float32x3(positions) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!()
        };
        assert!((positions.iter().map(|p| p[1]).fold(0., f32::max) / SCALE - 3.).abs() < 1e-5);
    }

    #[test]
    fn original_surface_decodes_as_a_warm_320_by_128_cry_image() {
        let image = surface_image();
        assert_eq!((image.width(), image.height()), (320, 128));
        let pixels = image.data.as_ref().unwrap();
        assert_eq!(pixels.len(), 320 * 128 * 4);
        let mut total = [0_u64; 3];
        for (raw, rgba) in SURFACE_CRY
            .as_chunks::<2>()
            .0
            .iter()
            .zip(pixels.as_chunks::<4>().0)
        {
            let expected = cry::color(raw[0], raw[1]).to_srgba();
            for (actual, channel) in
                rgba[..3]
                    .iter()
                    .zip([expected.red, expected.green, expected.blue])
            {
                assert!((f32::from(*actual) - channel * 255.).abs() < 0.001);
            }
            assert_eq!(rgba[3], 255);
            for channel in 0..3 {
                total[channel] += u64::from(rgba[channel]);
            }
        }
        assert!(
            total[0] > total[1] && total[1] > total[2],
            "surface should retain its red/gold palette"
        );
    }

    #[test]
    fn surface_stays_at_the_original_zero_height_and_scrolls_its_texture() {
        use bevy::mesh::VertexAttributeValues;
        let mesh = ground_mesh(0., 0);
        let VertexAttributeValues::Float32x3(positions) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!()
        };
        assert!(positions.iter().all(|p| p[1] == 0.));
        assert!(positions.iter().any(|p| p[2] < -190.));
        let later = ground_mesh(10., 0);
        let VertexAttributeValues::Float32x2(before) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap()
        else {
            panic!()
        };
        let VertexAttributeValues::Float32x2(after) =
            later.attribute(Mesh::ATTRIBUTE_UV_0).unwrap()
        else {
            panic!()
        };
        assert!((after[0][1] - before[0][1] - 10. / 12.8).abs() < 1e-6);
    }

    #[test]
    fn rendering_owns_bonus_camera_visibility_and_original_gate_geometry() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Game>()
            .init_resource::<Level>()
            .add_systems(Startup, setup)
            .add_systems(Update, draw);
        let camera = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                Projection::Perspective(PerspectiveProjection::default()),
                Transform::default(),
            ))
            .id();
        let mut session = Session::new(ZoneKind::GateFlight, 0, Some(1), 0);
        session.gates.push_back(Gate {
            index: 0,
            distance: 200 * 65536,
            age: 16,
        });
        let mut game = app.world_mut().resource_mut::<Game>();
        game.status = Status::Bonus;
        game.bonus = Some(session);
        app.update();
        let Projection::Perspective(p) = app.world().get::<Projection>(camera).unwrap() else {
            panic!()
        };
        assert_eq!((p.fov, p.near), (1.2, 0.01));
        let rotation = app
            .world_mut()
            .query::<(&WarpNode, &Transform)>()
            .iter(app.world())
            .find_map(|(node, pose)| matches!(node, WarpNode::Gate(0)).then_some(pose.rotation))
            .unwrap();
        assert!(
            (rotation * Vec3::X).distance(Vec3::new((TAU / 16.).cos(), -(TAU / 16.).sin(), 0.))
                < 1e-6
        );
        let assets = app.world().resource::<WarpAssets>();
        assert_eq!(
            app.world()
                .resource::<Assets<Mesh>>()
                .get(&assets.gates[0])
                .unwrap()
                .count_vertices(),
            48
        );
        assert!(
            app.world_mut()
                .query::<(&WarpNode, &Visibility)>()
                .iter(app.world())
                .any(|(node, visibility)| matches!(node, WarpNode::Gate(0))
                    && *visibility == Visibility::Visible)
        );
        app.world_mut().resource_mut::<Game>().status = Status::Entering;
        app.update();
        assert!(
            app.world_mut()
                .query::<(&WarpNode, &Visibility)>()
                .iter(app.world())
                .all(|(_, visibility)| *visibility == Visibility::Hidden)
        );
    }
}
