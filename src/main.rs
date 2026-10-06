use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::RenderLayers,
    core_pipeline::tonemapping::Tonemapping,
    mesh::PrimitiveTopology,
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
    window::PrimaryWindow,
};
use clap::Parser;
use std::path::PathBuf;

mod cli;
mod cry;
mod enemies;
mod explosion;
mod level;
mod stars;

use level::{Level, LevelActor, Levels, LoadReason};

use enemies::powerups::Upgrades;

#[cfg(test)]
const LANES: usize = 16;
const FRONT_RADIUS: f32 = 4.0;
const BACK_RADIUS: f32 = FRONT_RADIUS;
#[cfg(test)]
const DEPTH: f32 = 40.0 * FRONT_RADIUS / 6.0;
const WORLD_UNIT: f32 = FRONT_RADIUS / 24.0;
const LANE_UNIT: i32 = 1 << 16;
#[cfg(test)]
const WEB_PERIOD: i32 = LANES as i32 * LANE_UNIT;
const ACCELERATION: i32 = 0x400;
const MAX_VELOCITY: i32 = 0x6000;
#[cfg(test)]
const START_PROGRESS: i32 = 7 * LANE_UNIT;
const CAMERA_STEP: f32 = FRONT_RADIUS / 96.0;
const FIRE_INTERVAL: u8 = 4;
const MAX_BULLETS: usize = 8;
const BULLET_STEP: f32 = 3.5 * FRONT_RADIUS / 24.0;

#[derive(Component, Default)]
struct Glider {
    progress: i32,
    velocity: i32,
}

#[derive(Component, Default)]
struct Weapon {
    cooldown: u8,
    firing: bool,
}

#[derive(Component)]
#[require(LevelActor)]
struct Bullet {
    lane: usize,
}

/// A player shot reads as a white-hot core inside a cyan-green glow. The
/// original draws a small rotating vector shape and lets bloom spread the
/// colour outwards, so these layers stand in for that glow. The shades are
/// sampled from a reference screenshot of the Jaguar original.
fn shot_mesh() -> Mesh {
    const SEGMENTS: usize = 8;
    const THICKNESS: f32 = 0.3;
    let shades = [
        Color::srgb_u8(236, 250, 254),
        Color::srgb_u8(14, 250, 155),
        Color::srgb_u8(0, 154, 53),
    ];
    let radii = [0.037, 0.08, 0.132];
    let point = |layer: usize, index: usize, z: f32| {
        let angle = index as f32 * std::f32::consts::TAU / SEGMENTS as f32;
        Vec3::new(
            angle.cos() * radii[layer],
            angle.sin() * radii[layer] * 1.05,
            z,
        )
    };

    let mut triangles = Vec::new();
    for z in [THICKNESS, -THICKNESS] {
        for index in 0..SEGMENTS {
            let next = (index + 1) % SEGMENTS;
            let (a, b) = (point(0, index, z), point(0, next, z));
            triangles.push([
                (Vec3::new(0.0, 0.0, z), shades[0]),
                (a, shades[0]),
                (b, shades[0]),
            ]);
            for layer in 0..radii.len() - 1 {
                let (near, far) = (shades[layer], shades[layer + 1]);
                let (a, b) = (point(layer, index, z), point(layer, next, z));
                let (c, d) = (point(layer + 1, index, z), point(layer + 1, next, z));
                triangles.push([(a, near), (b, near), (c, far)]);
                triangles.push([(a, near), (c, far), (d, far)]);
            }
        }
    }
    // Close the rim so the bolt stays solid when a shot is seen from the side.
    for index in 0..SEGMENTS {
        let next = (index + 1) % SEGMENTS;
        let (a, b) = (point(2, index, THICKNESS), point(2, next, THICKNESS));
        let (c, d) = (point(2, index, -THICKNESS), point(2, next, -THICKNESS));
        triangles.push([(a, shades[2]), (c, shades[2]), (d, shades[2])]);
        triangles.push([(a, shades[2]), (d, shades[2]), (b, shades[2])]);
    }
    colored_mesh(triangles)
}

#[derive(Resource, Default)]
struct ShotAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

#[derive(Component, Default)]
struct ParticleShot(Option<Handle<Mesh>>);

fn particle_shot_mesh(
    position: Vec3,
    camera: &Transform,
    projection: &PerspectiveProjection,
) -> Mesh {
    let relative = camera.rotation.inverse() * (position - camera.translation);
    let distance = (-relative.z).max(0.01);
    let depth = (camera.translation.z - position.z) / (FRONT_RADIUS / 24.0);
    let phase = depth as i32 as u8;
    let color = cry::color(
        explosion::pulse_hue(phase),
        (255.0 * (1.0 - depth / 325.0).clamp(0.0, 1.0)) as u8,
    );
    let dot = 4.0 * distance * (projection.fov * 0.5).tan() * projection.aspect_ratio / 384.0;
    // draw_pring supplies radius 8; pring's sine multiplication halves it.
    colored_mesh((0..4).flat_map(|index| {
        let angle = (phase as f32 + index as f32 * 64.0) * std::f32::consts::TAU / 256.0;
        let center = Vec3::new(angle.sin(), angle.cos(), 0.0) * (4.0 * FRONT_RADIUS / 24.0);
        let quad = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(x, y)| {
            (
                camera.rotation * (center + Vec3::new(x * dot * 0.5, y * dot * 0.5, 0.0)),
                color,
            )
        });
        [[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]]
    }))
}

#[allow(clippy::type_complexity)]
fn draw_particle_shots(
    mut meshes: ResMut<Assets<Mesh>>,
    cameras: Query<(&Transform, &Projection), (With<Camera3d>, Without<ParticleShot>)>,
    mut shots: Query<(&mut ParticleShot, &mut Mesh3d, &Transform)>,
) {
    let Some((camera, Projection::Perspective(projection))) = cameras.iter().next() else {
        return;
    };
    for (mut particle, mut mesh, transform) in &mut shots {
        let geometry = particle_shot_mesh(transform.translation, camera, projection);
        if let Some(handle) = &particle.0 {
            if let Some(existing) = meshes.get_mut(handle) {
                *existing = geometry;
            }
        } else {
            particle.0 = Some(meshes.add(geometry));
        }
        mesh.0 = particle.0.as_ref().unwrap().clone();
    }
}

#[derive(Resource)]
struct ClawFrames([Handle<Mesh>; 8]);

#[derive(Component)]
struct LaneHighlight(usize);

#[derive(Resource)]
struct WebAssets {
    beam: Handle<Mesh>,
    blue: Handle<StandardMaterial>,
    yellow: Handle<StandardMaterial>,
    surface: Handle<StandardMaterial>,
}

#[derive(Component)]
struct LevelLabel;

#[derive(Resource)]
struct StartingLevel(usize);

#[derive(Resource)]
struct CapturePath(PathBuf);

#[derive(Resource)]
struct MotionCapture {
    directory: PathBuf,
    tick: u32,
    next_capture: u32,
    frame: u32,
    finished: bool,
    end_tick: u32,
    preview: CapturePreview,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CapturePreview {
    Motion,
    Exit,
    ShotHit,
}

fn main() {
    let args = cli::Cli::parse();
    let mut app = App::new();
    if let Some(path) = args.screenshot {
        app.insert_resource(CapturePath(path))
            .add_systems(Update, capture_frame);
    }
    let recording = args
        .record_motion
        .map(|directory| (directory, 6, CapturePreview::Motion))
        .or_else(|| {
            args.record_exit
                .map(|directory| (directory, 14, CapturePreview::Exit))
        })
        .or_else(|| {
            args.record_shot_hit
                .map(|directory| (directory, 15, CapturePreview::ShotHit))
        });
    if let Some((directory, default_seconds, preview)) = recording {
        std::fs::create_dir_all(&directory).expect("could not create motion capture directory");
        app.insert_resource(MotionCapture {
            directory,
            tick: 0,
            next_capture: 0,
            frame: 0,
            finished: false,
            end_tick: args.seconds.unwrap_or(default_seconds) * 60,
            preview,
        })
        .add_systems(FixedUpdate, preview_input.before(move_glider))
        .add_systems(Update, capture_motion);
    }
    let starting_level = args.level as usize;
    let levels = match Levels::load(&args.levels) {
        Ok(levels) => levels,
        Err(error) => {
            eprintln!("Could not load levels: {error}");
            std::process::exit(1);
        }
    };
    let initial = levels
        .0
        .get(starting_level - 1)
        .unwrap_or_else(|| {
            eprintln!(
                "Level {starting_level} does not exist; choose 1 through {}",
                levels.0.len()
            );
            std::process::exit(1);
        })
        .clone();
    app.insert_resource(initial)
        .insert_resource(levels)
        .insert_resource(StartingLevel(starting_level - 1));
    app.insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("T2K | Level {starting_level}"),
                resolution: (1100, 850).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup);
    enemies::add_systems(&mut app);
    stars::add_systems(&mut app);
    app.run();
}

#[cfg(test)]
fn rim_vertex(index: f32, radius: f32, z: f32) -> Vec3 {
    Level::default().web.vertex(index, radius, z)
}
#[cfg(test)]
fn selected_lane(progress: i32) -> f32 {
    Level::default().web.selected_lane(progress)
}
#[cfg(test)]
fn glider_transform(progress: i32) -> Transform {
    Level::default().web.glider_transform(progress)
}
#[cfg(test)]
fn camera_position(glider: Vec3) -> Vec3 {
    Level::default().web.camera_position(glider)
}

fn colored_mesh(triangles: impl IntoIterator<Item = [(Vec3, Color); 3]>) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    for triangle in triangles {
        let normal = (triangle[1].0 - triangle[0].0)
            .cross(triangle[2].0 - triangle[0].0)
            .normalize();
        for (position, color) in triangle {
            positions.push(position.to_array());
            normals.push(normal.to_array());
            colors.push(LinearRgba::from(color).to_f32_array());
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
}

fn tunnel_mesh(web: &level::Web, color: [f32; 3]) -> Mesh {
    let mut triangles = Vec::new();
    for lane in 0..web.lanes() {
        let lane = lane as f32;
        let extent = web
            .vertices
            .iter()
            .map(|[x, _]| ((x - 8.0) * FRONT_RADIUS / 6.0).abs())
            .fold(FRONT_RADIUS, f32::max);
        let left = 0.5 - web.vertex(lane, FRONT_RADIUS, 0.0).x / (2.0 * extent);
        for section in 0..12 {
            let mut quad = [(Vec3::ZERO, Color::BLACK); 4];
            for (vertex, (edge, depth)) in [
                (lane, section),
                (lane + 1.0, section),
                (lane + 1.0, section + 1),
                (lane, section + 1),
            ]
            .into_iter()
            .enumerate()
            {
                let progress = depth as f32 / 12.0;
                let blue = (0.08 + left * 0.5) * (1.0 - progress).powf(1.4) + 0.012;
                quad[vertex] = (
                    web.vertex(edge, FRONT_RADIUS, -web.world_depth() * progress),
                    Color::srgb(color[0] * blue, color[1] * blue, color[2] * blue),
                );
            }
            triangles.extend([[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]]);
        }
    }
    colored_mesh(triangles)
}

fn claw_mesh(frame: usize) -> Mesh {
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

fn segment_transform(start: Vec3, end: Vec3, width: f32) -> Transform {
    let direction = end - start;
    Transform::from_translation((start + end) * 0.5)
        .with_rotation(Quat::from_rotation_arc(Vec3::Y, direction.normalize()))
        .with_scale(Vec3::new(width, direction.length(), width))
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    level: Res<Level>,
) {
    let web = &level.web;
    enemies::setup(&mut commands, &mut meshes, &mut materials, &level);
    // Original actors render over the web rather than being clipped by its faces.
    for layer in 0..=1 {
        commands.spawn((
            Camera3d::default(),
            Camera {
                order: layer as isize,
                clear_color: if layer == 0 {
                    ClearColorConfig::Default
                } else {
                    ClearColorConfig::None
                },
                ..default()
            },
            RenderLayers::layer(layer),
            Tonemapping::None,
            Projection::Perspective(PerspectiveProjection {
                fov: 1.2,
                ..default()
            }),
            Transform::from_translation(
                web.camera_position(web.glider_transform(web.start()).translation),
            )
            .with_rotation(Quat::from_euler(EulerRot::YXZ, 0.105, 0.06, 0.0)),
        ));
    }

    let beam = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let [r, g, b] = level.appearance.lines;
    let blue = materials.add(StandardMaterial {
        base_color: Color::srgb(r, g, b),
        unlit: true,
        ..default()
    });
    let vertex_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let yellow = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.82, 0.04),
        unlit: true,
        ..default()
    });
    let (root, surface) = enemies::entry::setup(&mut commands, &mut meshes, &mut materials, web);
    commands.insert_resource(ShotAssets {
        mesh: meshes.add(shot_mesh()),
        material: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            cull_mode: None,
            ..default()
        }),
    });

    let assets = WebAssets {
        beam,
        blue,
        yellow,
        surface,
    };
    spawn_web(
        &mut commands,
        &mut meshes,
        root,
        web,
        &assets,
        level.appearance.surface,
    );
    commands.insert_resource(assets);
    let frames = ClawFrames(std::array::from_fn(|frame| meshes.add(claw_mesh(frame))));
    commands.spawn((
        Glider {
            progress: web.start(),
            velocity: 0,
        },
        Weapon::default(),
        Visibility::Hidden,
        RenderLayers::layer(1),
        web.glider_transform(web.start()),
        Mesh3d(frames.0[claw_frame(web.start())].clone()),
        MeshMaterial3d(vertex_material),
    ));
    commands.insert_resource(frames);

    stars::setup(&mut commands, &mut meshes, &mut materials);

    commands.spawn((
        LevelLabel,
        Text::new(format!("LEVEL {:02}", level.number)),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.5, 0.5, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            top: px(22),
            right: px(26),
            ..default()
        },
    ));
    commands.spawn((
        Text::new("LEFT / RIGHT or A / D  Move    ENTER / SPACE  Fire    J / SHIFT  Jump    R  Restart    ESC  Quit"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.4, 0.4, 0.5)),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(22),
            left: px(26),
            ..default()
        },
    ));
}

fn spawn_web(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    root: Entity,
    web: &level::Web,
    assets: &WebAssets,
    color: [f32; 3],
) {
    commands.spawn((
        Mesh3d(meshes.add(tunnel_mesh(web, color))),
        MeshMaterial3d(assets.surface.clone()),
        ChildOf(root),
    ));
    for lane in 0..web.lanes() {
        let lane = lane as f32;
        commands.spawn((
            ChildOf(root),
            Mesh3d(assets.beam.clone()),
            MeshMaterial3d(assets.blue.clone()),
            segment_transform(
                web.vertex(lane, FRONT_RADIUS, 0.0),
                web.vertex(lane, BACK_RADIUS, -web.world_depth()),
                0.024,
            ),
        ));
        for z in [0.0, -web.world_depth()] {
            commands.spawn((
                ChildOf(root),
                Mesh3d(assets.beam.clone()),
                MeshMaterial3d(assets.blue.clone()),
                segment_transform(
                    web.vertex(lane, FRONT_RADIUS, z),
                    web.vertex(lane + 1.0, FRONT_RADIUS, z),
                    0.026,
                ),
            ));
        }
    }
    for edge in 0..2 {
        commands.spawn((
            ChildOf(root),
            LaneHighlight(edge),
            Visibility::Hidden,
            Mesh3d(assets.beam.clone()),
            MeshMaterial3d(assets.yellow.clone()),
            Transform::default(),
        ));
    }
    if !web.closed {
        commands.spawn((
            ChildOf(root),
            Mesh3d(assets.beam.clone()),
            MeshMaterial3d(assets.blue.clone()),
            segment_transform(
                web.vertex(web.lanes() as f32, FRONT_RADIUS, 0.0),
                web.vertex(web.lanes() as f32, BACK_RADIUS, -web.world_depth()),
                0.024,
            ),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_web(
    mut commands: Commands,
    level: Res<Level>,
    assets: Res<WebAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    root: Single<Entity, With<enemies::entry::WebRoot>>,
    mut label: Single<&mut Text, With<LevelLabel>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !level.is_changed() {
        return;
    }
    commands.entity(*root).despawn_related::<Children>();
    spawn_web(
        &mut commands,
        &mut meshes,
        *root,
        &level.web,
        &assets,
        level.appearance.surface,
    );
    if let Some(material) = materials.get_mut(&assets.blue) {
        let [r, g, b] = level.appearance.lines;
        material.base_color = Color::srgb(r, g, b);
    }
    label.0 = format!("LEVEL {:02}", level.number);
    for mut window in &mut windows {
        window.title = format!("T2K | Level {}", level.number);
    }
}

fn claw_frame(progress: i32) -> usize {
    // dsclaw uses 16:16 fractional progress. WEB reverses the original lane order.
    ((-(progress + LANE_UNIT / 2)).rem_euclid(LANE_UNIT) >> 13) as usize
}

fn animate_glider(frames: Res<ClawFrames>, mut gliders: Query<(&Glider, &mut Mesh3d)>) {
    for (glider, mut mesh) in &mut gliders {
        let frame = &frames.0[claw_frame(glider.progress)];
        if mesh.0 != *frame {
            mesh.0 = frame.clone();
        }
    }
}

fn update_lane(
    level: Res<Level>,
    glider: Single<&Glider>,
    mut highlights: Query<(&LaneHighlight, &mut Transform), Without<Glider>>,
) {
    let web = &level.web;
    let lane = web.selected_lane(glider.progress);
    for (edge, mut transform) in &mut highlights {
        let index = lane + edge.0 as f32;
        *transform = segment_transform(
            web.vertex(index, FRONT_RADIUS, 0.0) + Vec3::Z * 0.025,
            web.vertex(index, BACK_RADIUS, -web.world_depth()) + Vec3::Z * 0.025,
            0.034,
        );
    }
}

fn follow_glider(
    level: Res<Level>,
    glider: Single<&Transform, With<Glider>>,
    mut cameras: Query<&mut Transform, (With<Camera3d>, Without<Glider>)>,
) {
    for mut transform in &mut cameras {
        let target = level.web.camera_position(glider.translation);
        let delta = target - transform.translation;
        transform.translation += delta.clamp(Vec3::splat(-CAMERA_STEP), Vec3::splat(CAMERA_STEP));
        transform.translation.z = target.z;
    }
}

fn fit_camera(
    level: Res<Level>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut cameras: Query<&mut Projection, With<Camera3d>>,
) {
    for mut projection in &mut cameras {
        if let Projection::Perspective(perspective) = &mut *projection {
            let aspect = window.width() / window.height().max(1.0);
            let [width, height] =
                level
                    .web
                    .vertices
                    .iter()
                    .fold([4.0_f32, 4.0_f32], |[w, h], [x, y]| {
                        [
                            w.max(((x - 8.0) * FRONT_RADIUS / 6.0).abs()),
                            h.max(((y - 8.0) * FRONT_RADIUS / 6.0).abs()),
                        ]
                    });
            let half_height = (5.0 * (1.0_f32.max(1.294 / aspect)))
                .max(height + 1.0)
                .max((width + 1.0) / aspect);
            perspective.fov = 2.0 * (half_height / 5.3).atan();
        }
    }
}

fn capture_frame(
    mut commands: Commands,
    path: Res<CapturePath>,
    mut frames: Local<u32>,
    glider: Single<&Visibility, With<Glider>>,
) {
    if *glider != Visibility::Visible {
        return;
    }
    *frames += 1;
    if *frames == 60 {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.0.clone()))
            .observe(
                |_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                    exit.write(AppExit::Success);
                },
            );
    }
}

#[allow(clippy::too_many_arguments)]
fn preview_input(
    level: Res<Level>,
    mut capture: ResMut<MotionCapture>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    player: Single<&Glider>,
    pickups: Query<&enemies::powerups::PowerUp>,
    enemies: Query<&enemies::Flipper>,
    tankers: Query<&enemies::tankers::Tanker>,
    upgrades: Res<Upgrades>,
) {
    keys.release(KeyCode::KeyD);
    keys.release(KeyCode::KeyA);
    keys.release(KeyCode::Space);
    keys.release(KeyCode::KeyJ);
    if upgrades.jump_enabled && pickups.is_empty() && capture.tick.is_multiple_of(120) {
        keys.press(KeyCode::KeyJ);
    }
    if (30..capture.end_tick.saturating_sub(30)).contains(&capture.tick) {
        keys.press(KeyCode::Space);
    }
    let target = pickups
        .iter()
        .find_map(|pickup| pickup.preview_lane())
        .or_else(|| {
            enemies
                .iter()
                .filter_map(|enemy| enemy.preview_target(&level.web))
                .chain(
                    tankers
                        .iter()
                        .filter(|t| !t.approach)
                        .map(|t| (t.lane, t.depth)),
                )
                .min_by_key(|(_, depth)| *depth)
                .map(|(lane, _)| lane)
        });
    if let Some(target) = target {
        let lane = level.web.selected_lane(player.progress) as usize;
        let lanes = level.web.lanes();
        let clockwise = if level.web.closed {
            (target + lanes - lane) % lanes
        } else {
            target.abs_diff(lane)
        };
        if clockwise > 0 {
            keys.press(
                if (level.web.closed && clockwise < lanes / 2)
                    || (!level.web.closed && target > lane)
                {
                    KeyCode::KeyA
                } else {
                    KeyCode::KeyD
                },
            );
        }
    } else {
        match capture.tick % 360 {
            60..90 | 240..300 => keys.press(KeyCode::KeyD),
            150..180 => keys.press(KeyCode::KeyA),
            _ => {}
        }
    }
    capture.tick += 1;
}

fn capture_motion(mut commands: Commands, mut capture: ResMut<MotionCapture>) {
    if capture.finished || capture.tick < capture.next_capture {
        return;
    }
    let finished = capture.tick >= capture.end_tick;
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(
            capture.directory.join(format!("{:04}.png", capture.frame)),
        ))
        .observe(
            move |_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                if finished {
                    exit.write(AppExit::Success);
                }
            },
        );
    capture.frame += 1;
    capture.next_capture = capture.tick + 2;
    capture.finished = finished;
}

fn move_glider(
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

fn fire_bullets(
    level: Res<Level>,
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    assets: Res<ShotAssets>,
    mut glider: Single<(&Glider, &mut Weapon, &Transform)>,
    bullets: Query<&Bullet>,
    upgrades: Option<Res<Upgrades>>,
) {
    let (glider, weapon, transform) = &mut *glider;
    if !keys.any_pressed([KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Space]) {
        weapon.firing = false;
        return;
    }
    weapon.cooldown = weapon.cooldown.saturating_sub(1);
    if weapon.cooldown != 0 {
        return;
    }
    weapon.cooldown = if upgrades.as_ref().is_some_and(|u| u.laser) {
        3
    } else {
        FIRE_INTERVAL
    };
    if bullets.iter().count()
        >= MAX_BULLETS
            + if upgrades.as_ref().is_some_and(|u| u.droid) {
                8
            } else {
                0
            }
    {
        weapon.firing = false;
        return;
    }
    weapon.firing = true;
    let lane = level.web.selected_lane(glider.progress) as usize;
    let position = transform.translation - Vec3::Z * 0.18;
    let mut shot = commands.spawn((
        Bullet { lane },
        RenderLayers::layer(1),
        Mesh3d(assets.mesh.clone()),
        MeshMaterial3d(assets.material.clone()),
        Transform::from_translation(position),
    ));
    if upgrades.as_ref().is_some_and(|u| u.laser) {
        shot.insert(ParticleShot::default());
    }
}

fn move_bullets(
    level: Res<Level>,
    mut commands: Commands,
    mut bullets: Query<(Entity, &Bullet, &mut Transform)>,
    upgrades: Option<Res<Upgrades>>,
) {
    for (entity, bullet, mut transform) in &mut bullets {
        let center = level
            .web
            .glider_transform(bullet.lane as i32 * LANE_UNIT)
            .translation;
        transform.translation.x = center.x;
        transform.translation.y = center.y;
        transform.translation.z -= if upgrades.as_ref().is_some_and(|u| u.laser) {
            4.5 * FRONT_RADIUS / 24.0
        } else {
            BULLET_STEP
        };
        if transform.translation.z <= -level.web.world_depth() {
            commands.entity(entity).despawn();
        }
    }
}

fn quit_on_escape(keys: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn movement_app(progress: i32, keys: &[KeyCode]) -> App {
        let mut app = App::new();
        let mut input = ButtonInput::<KeyCode>::default();
        for &key in keys {
            input.press(key);
        }
        app.init_resource::<Level>()
            .insert_resource(input)
            .add_systems(FixedUpdate, move_glider);
        app.world_mut().spawn((
            Glider {
                progress,
                velocity: 0,
            },
            glider_transform(progress),
        ));
        app
    }

    fn state(app: &mut App) -> (i32, i32) {
        let glider = app
            .world_mut()
            .query::<&Glider>()
            .single(app.world())
            .unwrap();
        (glider.progress, glider.velocity)
    }

    fn ticks(app: &mut App, count: usize) {
        for _ in 0..count {
            app.world_mut().run_schedule(FixedUpdate);
        }
    }

    #[test]
    fn movement_stops_at_both_open_web_endpoints() {
        let level = Level::parse(include_str!("../levels/03.toml"), "level3").unwrap();
        for (progress, key) in [(0, KeyCode::KeyD), (10 * LANE_UNIT, KeyCode::KeyA)] {
            let mut app = movement_app(progress, &[key]);
            app.insert_resource(level.clone());
            ticks(&mut app, 30);
            assert_eq!(state(&mut app), (progress, 0));
        }
    }

    #[test]
    fn movement_uses_the_active_webs_lane_count() {
        let mut app = movement_app(512, &[KeyCode::ArrowRight]);
        let mut level = app.world_mut().resource_mut::<Level>();
        level.web.vertices.truncate(8);
        level.web.orientations.truncate(8);
        ticks(&mut app, 1);
        assert_eq!(state(&mut app), (8 * LANE_UNIT - 512, -ACCELERATION));
    }

    #[test]
    fn movement_wraps_in_both_directions() {
        let mut right = movement_app(512, &[KeyCode::ArrowRight]);
        ticks(&mut right, 1);
        assert_eq!(state(&mut right), (WEB_PERIOD - 512, -ACCELERATION));

        let mut left = movement_app(WEB_PERIOD - 512, &[KeyCode::KeyA]);
        ticks(&mut left, 1);
        assert_eq!(state(&mut left), (512, ACCELERATION));
    }

    #[test]
    fn right_follows_the_original_web5_from_its_first_lane_to_its_second() {
        // Source lane zero is between (7,2) and (5,3), our lane 15.
        // Its next lane is between (5,3) and (3,5), with midpoint (4,4).
        for key in [KeyCode::ArrowRight, KeyCode::KeyD] {
            let mut app = movement_app(15 * LANE_UNIT, &[key]);
            ticks(&mut app, 8);
            let (progress, _) = state(&mut app);
            let position = glider_transform(progress).translation;
            assert!(position.distance(Vec3::new(-8.0 / 3.0, 8.0 / 3.0, 0.12)) < 1e-5);
        }
        for key in [KeyCode::ArrowLeft, KeyCode::KeyA] {
            let mut app = movement_app(15 * LANE_UNIT, &[key]);
            ticks(&mut app, 8);
            let (progress, _) = state(&mut app);
            assert!(
                glider_transform(progress)
                    .translation
                    .distance(Vec3::new(0.0, 4.0, 0.12))
                    < 1e-5
            );
        }
    }

    #[test]
    fn movement_accelerates_to_original_speed_and_stops_before_reversing() {
        let mut app = movement_app(0, &[KeyCode::KeyA]);
        ticks(&mut app, 1);
        assert_eq!(state(&mut app), (0x400, 0x400));
        ticks(&mut app, 23);
        assert_eq!(state(&mut app), (307_200, MAX_VELOCITY));
        ticks(&mut app, 1);
        assert_eq!(state(&mut app), (331_776, MAX_VELOCITY));

        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyA);
        keys.press(KeyCode::KeyD);
        ticks(&mut app, 1);
        assert_eq!(state(&mut app), (331_776, 0));
        ticks(&mut app, 1);
        assert_eq!(state(&mut app), (330_752, -ACCELERATION));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyD);
        ticks(&mut app, 10);
        assert_eq!(state(&mut app), (330_752, 0));

        let mut opposed = movement_app(3 * LANE_UNIT, &[KeyCode::ArrowLeft, KeyCode::KeyD]);
        ticks(&mut opposed, 10);
        assert_eq!(state(&mut opposed), (3 * LANE_UNIT, 0));
    }

    #[test]
    fn fixed_simulation_is_independent_of_render_frame_rate() {
        use bevy::time::TimeUpdateStrategy;

        let mut slow = movement_app(0, &[KeyCode::KeyA]);
        let mut fast = movement_app(0, &[KeyCode::KeyA]);
        for (app, nanos) in [(&mut slow, 33_333_334), (&mut fast, 16_666_667)] {
            app.add_plugins(MinimalPlugins)
                .insert_resource(Time::<Fixed>::from_hz(60.0))
                .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_nanos(
                    nanos,
                )));
            app.update();
        }
        for _ in 0..30 {
            slow.update();
        }
        for _ in 0..60 {
            fast.update();
        }
        assert_eq!(state(&mut slow), state(&mut fast));
        assert_eq!(state(&mut slow), (143_360, MAX_VELOCITY));
    }

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
                (glider_transform(edge as i32 * LANE_UNIT).rotation * Vec3::X).distance(tangent)
                    < 1e-5
            );
        }
    }

    #[test]
    fn claw_animation_tracks_movement_reversal_and_stopping() {
        let mut app = movement_app(0, &[KeyCode::KeyA]);
        let mut meshes = Assets::<Mesh>::default();
        let frames = ClawFrames(std::array::from_fn(|frame| meshes.add(claw_mesh(frame))));
        let ids = frames.0.clone();
        let glider = app
            .world_mut()
            .query::<Entity>()
            .single(app.world())
            .unwrap();
        app.world_mut()
            .entity_mut(glider)
            .insert(Mesh3d(ids[4].clone()));
        app.insert_resource(frames)
            .add_systems(FixedUpdate, animate_glider.after(move_glider));
        let frame = |app: &App| app.world().get::<Mesh3d>(glider).unwrap().0.clone();

        ticks(&mut app, 1);
        assert_eq!(frame(&app), ids[3]);
        ticks(&mut app, 5);
        assert_eq!(frame(&app), ids[1]);
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release(KeyCode::KeyA);
        input.press(KeyCode::KeyD);
        ticks(&mut app, 1);
        assert_eq!(frame(&app), ids[1]);
        ticks(&mut app, 6);
        assert_eq!(frame(&app), ids[4]);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyD);
        ticks(&mut app, 10);
        assert_eq!(frame(&app), ids[4]);
    }

    #[test]
    fn claw_animation_repeats_each_lane_and_across_the_web_seam() {
        for step in 0..8 {
            let phase = -(step as i32 * (LANE_UNIT / 8) + LANE_UNIT / 2);
            for lane in 0..LANES {
                assert_eq!(claw_frame(lane as i32 * LANE_UNIT + phase), step);
            }
            assert_eq!(claw_frame(phase), claw_frame(WEB_PERIOD + phase));
        }
    }

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

    #[test]
    fn camera_settles_after_the_claw_stops_without_overshooting() {
        let mut app = movement_app(8 * LANE_UNIT, &[]);
        app.add_systems(FixedUpdate, follow_glider.after(move_glider));
        let start = camera_position(glider_transform(START_PROGRESS).translation);
        let target = camera_position(glider_transform(8 * LANE_UNIT).translation);
        let camera = app
            .world_mut()
            .spawn((Camera3d::default(), Transform::from_translation(start)))
            .id();
        ticks(&mut app, 1);
        let first = app.world().get::<Transform>(camera).unwrap().translation;
        assert!(first.distance(target) < start.distance(target));
        assert!((first - start).abs().max_element() <= CAMERA_STEP + 1e-6);
        ticks(&mut app, 100);
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            target
        );
        assert_eq!(state(&mut app), (8 * LANE_UNIT, 0));
    }

    #[test]
    fn particle_rings_have_four_separate_screen_sized_dots_and_change_color_with_depth() {
        let camera = Transform::from_translation(Vec3::new(0.0, 0.0, 5.3))
            .with_rotation(Quat::from_rotation_y(0.1));
        let projection = PerspectiveProjection {
            fov: 1.2,
            aspect_ratio: 1.294,
            ..default()
        };
        let mut shades = Vec::new();
        for z in [0.0, -10.0] {
            let position = Vec3::new(0.0, 0.0, z);
            let mesh = particle_shot_mesh(position, &camera, &projection);
            let vertices = mesh
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .unwrap()
                .as_float3()
                .unwrap();
            assert_eq!(vertices.len(), 24);
            for dot in vertices.chunks_exact(6) {
                let local = |index| camera.rotation.inverse() * Vec3::from_array(dot[index]);
                let width = (local(1) - local(0)).length();
                let distance = -(camera.rotation.inverse() * (position - camera.translation)).z;
                let pixels = width * 384.0
                    / (2.0 * distance * (projection.fov * 0.5).tan() * projection.aspect_ratio);
                assert!((pixels - 2.0).abs() < 1e-4);
                let center = (local(0) + local(2)) * 0.5;
                assert!((center.length() - 4.0 * FRONT_RADIUS / 24.0).abs() < 1e-5);
            }
            let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
                mesh.attribute(Mesh::ATTRIBUTE_COLOR)
            else {
                panic!("missing particle colors");
            };
            shades.push(colors[0]);
        }
        assert_ne!(shades[0], shades[1]);
    }

    #[test]
    fn shot_has_a_white_core_inside_a_cyan_green_glow() {
        let mesh = shot_mesh();
        let colors = match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
            Some(bevy::mesh::VertexAttributeValues::Float32x4(values)) => values,
            other => panic!("shot mesh needs colours, got {other:?}"),
        };
        assert!(!colors.is_empty());

        // Exactly the three sampled shades, in the linear space the GPU uses.
        let shade = |rgb: [u8; 3]| {
            let value = LinearRgba::from(Color::srgb_u8(rgb[0], rgb[1], rgb[2])).to_f32_array();
            [value[0], value[1], value[2]]
        };
        let shades = [
            shade([236, 250, 254]),
            shade([14, 250, 155]),
            shade([0, 154, 53]),
        ];
        let matches = |color: &[f32], want: &[f32; 3]| {
            (0..3).all(|channel| (color[channel] - want[channel]).abs() < 1e-6)
        };
        for color in colors {
            assert!(
                shades.iter().any(|want| matches(color, want)),
                "unexpected shot colour {color:?}"
            );
        }
        for want in shades {
            assert!(colors.iter().any(|color| matches(color, &want)));
        }
        // The core is the brightest layer and the rim the dimmest.
        let luminance = |shade: &[f32; 3]| shade.iter().sum::<f32>();
        assert!(luminance(&shades[0]) > luminance(&shades[1]));
        assert!(luminance(&shades[1]) > luminance(&shades[2]));
    }

    fn firing_app(progress: i32, keys: &[KeyCode]) -> App {
        let mut app = movement_app(progress, keys);
        let glider = app
            .world_mut()
            .query_filtered::<Entity, With<Glider>>()
            .single(app.world())
            .unwrap();
        app.world_mut().entity_mut(glider).insert(Weapon::default());
        app.insert_resource(ShotAssets::default()).add_systems(
            FixedUpdate,
            (move_bullets, fire_bullets).chain().after(move_glider),
        );
        app
    }

    fn bullet_count(app: &mut App) -> usize {
        app.world_mut().query::<&Bullet>().iter(app.world()).count()
    }

    #[test]
    fn enter_fires_immediately_and_repeats_at_the_weapon_cadence() {
        let mut app = firing_app(START_PROGRESS, &[]);
        ticks(&mut app, 1);
        assert_eq!(bullet_count(&mut app), 0);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        ticks(&mut app, 1);
        assert_eq!(bullet_count(&mut app), 1);
        ticks(&mut app, 3);
        assert_eq!(bullet_count(&mut app), 1);
        ticks(&mut app, 1);
        assert_eq!(bullet_count(&mut app), 2);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Enter);
        ticks(&mut app, 8);
        assert_eq!(bullet_count(&mut app), 2);
        assert!(
            !app.world_mut()
                .query::<&Weapon>()
                .single(app.world())
                .unwrap()
                .firing
        );
    }

    #[test]
    fn a_full_magazine_skips_a_shot_instead_of_firing_as_soon_as_a_slot_frees() {
        let mut app = firing_app(START_PROGRESS, &[KeyCode::Enter]);
        let slots: Vec<_> = (0..MAX_BULLETS)
            .map(|_| {
                app.world_mut()
                    .spawn((Bullet { lane: 7 }, Transform::default()))
                    .id()
            })
            .collect();
        ticks(&mut app, 1);
        app.world_mut().despawn(slots[0]);
        ticks(&mut app, 3);
        assert_eq!(bullet_count(&mut app), MAX_BULLETS - 1);
        assert!(
            !app.world_mut()
                .query::<&Weapon>()
                .single(app.world())
                .unwrap()
                .firing
        );
        ticks(&mut app, 1);
        assert_eq!(bullet_count(&mut app), MAX_BULLETS);
        assert!(
            app.world_mut()
                .query::<&Weapon>()
                .single(app.world())
                .unwrap()
                .firing
        );
    }

    #[test]
    fn particle_laser_increases_fire_rate_and_shot_speed() {
        let mut app = firing_app(START_PROGRESS, &[KeyCode::Space]);
        let mut upgrades = Upgrades::default();
        upgrades.laser = true;
        app.insert_resource(upgrades);
        ticks(&mut app, 1);
        let shot = app
            .world_mut()
            .query_filtered::<Entity, With<Bullet>>()
            .single(app.world())
            .unwrap();
        let start = app.world().get::<Transform>(shot).unwrap().translation.z;
        ticks(&mut app, 3);
        assert_eq!(bullet_count(&mut app), 2);
        let position = app.world().get::<Transform>(shot).unwrap().translation.z;
        assert!((start - position - 3.0 * 4.5 * FRONT_RADIUS / 24.0).abs() < 1e-5);
    }

    #[test]
    fn shots_keep_their_launch_lane_while_the_claw_moves_and_wraps() {
        let mut app = firing_app(WEB_PERIOD - 512, &[KeyCode::Space, KeyCode::ArrowLeft]);
        ticks(&mut app, 1);
        let shot = app
            .world_mut()
            .query_filtered::<Entity, With<Bullet>>()
            .single(app.world())
            .unwrap();
        assert_eq!(app.world().get::<Bullet>(shot).unwrap().lane, 0);
        let start = app.world().get::<Transform>(shot).unwrap().translation;
        assert_eq!(start.truncate(), glider_transform(0).translation.truncate());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Space);
        ticks(&mut app, 16);
        assert_ne!(selected_lane(state(&mut app).0), 0.0);
        let position = app.world().get::<Transform>(shot).unwrap().translation;
        assert_eq!(position.truncate(), start.truncate());
        assert!((position.z - (start.z - 16.0 * BULLET_STEP)).abs() < 1e-4);
    }

    #[test]
    fn sustained_fire_is_bounded_and_shots_are_cleaned_up() {
        let mut app = firing_app(START_PROGRESS, &[KeyCode::Space]);
        let mut reached_limit = false;
        for _ in 0..180 {
            ticks(&mut app, 1);
            let count = bullet_count(&mut app);
            assert!(count <= MAX_BULLETS);
            reached_limit |= count == MAX_BULLETS;
        }
        assert!(reached_limit);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Space);
        ticks(&mut app, 60);
        assert_eq!(bullet_count(&mut app), 0);
        assert_eq!(
            app.world_mut().query::<&Mesh3d>().iter(app.world()).count(),
            0
        );
    }

    #[test]
    fn repeat_fire_is_independent_of_render_frame_rate() {
        use bevy::time::TimeUpdateStrategy;

        let mut slow = firing_app(0, &[KeyCode::Space]);
        let mut fast = firing_app(0, &[KeyCode::Space]);
        for (app, nanos) in [(&mut slow, 33_333_334), (&mut fast, 16_666_667)] {
            app.add_plugins(MinimalPlugins)
                .insert_resource(Time::<Fixed>::from_hz(60.0))
                .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_nanos(
                    nanos,
                )));
            app.update();
        }
        for _ in 0..30 {
            slow.update();
        }
        for _ in 0..60 {
            fast.update();
        }
        let positions = |app: &mut App| {
            let mut depths: Vec<_> = app
                .world_mut()
                .query_filtered::<&Transform, With<Bullet>>()
                .iter(app.world())
                .map(|transform| transform.translation.z)
                .collect();
            depths.sort_by(f32::total_cmp);
            depths
        };
        assert_eq!(positions(&mut slow), positions(&mut fast));
    }
}
