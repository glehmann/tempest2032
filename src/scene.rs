//! Scene startup, web surfaces and wireframes, and selected-lane highlights.
//! Rebuilds the visible web when the active level changes.

use bevy::{
    camera::visibility::RenderLayers, core_pipeline::tonemapping::Tonemapping, prelude::*,
    window::PrimaryWindow,
};

use crate::{
    animations::level_entry as entry,
    enemies,
    glider::{ClawFrames, Glider, claw_frame, claw_mesh},
    level::{Appearance, Level},
    mesh::{colored_mesh, segment_transform},
    stars,
    weapons::{ShotAssets, Weapon, shot_mesh},
    web::{BACK_RADIUS, FRONT_RADIUS, Web},
};

#[derive(Component)]
pub(crate) struct LaneHighlight(pub(crate) usize);

#[derive(Component)]
pub(crate) struct WebSegment {
    pub(crate) lane: usize,
    pub(crate) rim: bool,
}

#[derive(Component)]
pub(crate) struct WebSurface {
    hues: Vec<u8>,
    depth: f32,
}

#[derive(Resource)]
pub(crate) struct WebAssets {
    pub(crate) beam: Handle<Mesh>,
    pub(crate) blue: Handle<StandardMaterial>,
    pub(crate) yellow: Handle<StandardMaterial>,
    pub(crate) surface: Handle<StandardMaterial>,
}

const SURFACE_SECTIONS: usize = 12;

fn surface_color(web: &Web, appearance: &Appearance, lane: usize, depth: f32, frame: u8) -> Color {
    if let Some(hues) = &appearance.surface_hues {
        return lane_color(hues, lane, depth, frame);
    }
    let extent = web
        .vertices
        .iter()
        .map(|[x, _]| ((x - 8.) * FRONT_RADIUS / 6.).abs())
        .fold(FRONT_RADIUS, f32::max);
    let left = 0.5 - web.vertex(lane as f32, FRONT_RADIUS, 0.).x / (2. * extent);
    let shade = (0.08 + left * 0.5) * (1. - depth).powf(1.4) + 0.012;
    let [r, g, b] = appearance.surface.map(|channel| channel * shade);
    Color::srgb(r, g, b)
}

fn lane_color(hues: &[u8], lane: usize, depth: f32, frame: u8) -> Color {
    // donky.gas overwrites the orientation-table phase with the previous
    // face's hue. Our reversed lanes put that face at lane + 1.
    let phase = i32::from(hues[(lane + 1) % hues.len()]) + i32::from(frame) + 64;
    let front = ((crate::sines::sine(phase) + 128) / 2 + 64) * 248 / 256;
    crate::cry::color(hues[lane], (front as f32 * (1. - depth * 0.75)) as u8)
}

pub(crate) fn tunnel_mesh(web: &Web, appearance: &Appearance) -> Mesh {
    if appearance.surface_hues.as_ref().is_some_and(Vec::is_empty) {
        return colored_mesh([]);
    }
    let mut triangles = Vec::new();
    for lane in 0..web.lanes() {
        for section in 0..SURFACE_SECTIONS {
            let mut quad = [(Vec3::ZERO, Color::BLACK); 4];
            for (vertex, (edge, depth)) in [
                (lane as f32, section),
                (lane as f32 + 1.0, section),
                (lane as f32 + 1.0, section + 1),
                (lane as f32, section + 1),
            ]
            .into_iter()
            .enumerate()
            {
                let progress = depth as f32 / SURFACE_SECTIONS as f32;
                quad[vertex] = (
                    web.vertex(edge, FRONT_RADIUS, -web.world_depth() * progress),
                    surface_color(web, appearance, lane, progress, 0),
                );
            }
            triangles.extend([[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]]);
        }
    }
    colored_mesh(triangles)
}

pub(crate) fn animate_surface(
    surfaces: Query<(&Mesh3d, &WebSurface)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut frame: Local<u8>,
) {
    *frame = frame.wrapping_add(1);
    for (handle, surface) in &surfaces {
        let Some(mesh) = meshes.get_mut(&handle.0) else {
            continue;
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(points)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            continue;
        };
        let colors = points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                lane_color(
                    &surface.hues,
                    index / (SURFACE_SECTIONS * 6),
                    -point[2] / surface.depth,
                    *frame,
                )
                .to_linear()
                .to_f32_array()
            })
            .collect::<Vec<_>>();
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
}

pub(crate) fn setup(
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
            crate::camera::WebCamera,
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
            .with_rotation(web.camera_rotation()),
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
    let (root, surface) = entry::setup(&mut commands, &mut meshes, &mut materials, web);
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
        &level.appearance,
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
}

pub(crate) fn spawn_web(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    root: Entity,
    web: &Web,
    assets: &WebAssets,
    appearance: &Appearance,
) {
    if !appearance.surface_hues.as_ref().is_some_and(Vec::is_empty) {
        let mut surface = commands.spawn((
            Mesh3d(meshes.add(tunnel_mesh(web, appearance))),
            MeshMaterial3d(assets.surface.clone()),
            ChildOf(root),
        ));
        if let Some(hues) = &appearance.surface_hues {
            surface.insert(WebSurface {
                hues: hues.clone(),
                depth: web.world_depth(),
            });
        }
    }
    for lane in 0..web.lanes() {
        let lane = lane as f32;
        commands.spawn((
            WebSegment {
                lane: lane as usize,
                rim: false,
            },
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
            let mut segment = commands.spawn((
                ChildOf(root),
                Mesh3d(assets.beam.clone()),
                MeshMaterial3d(assets.blue.clone()),
                segment_transform(
                    web.vertex(lane, FRONT_RADIUS, z),
                    web.vertex(lane + 1.0, FRONT_RADIUS, z),
                    0.026,
                ),
            ));
            if z == 0.0 {
                segment.insert(WebSegment {
                    lane: lane as usize,
                    rim: true,
                });
            }
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
            WebSegment {
                lane: web.lanes(),
                rim: false,
            },
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
pub(crate) fn sync_web(
    mut commands: Commands,
    level: Res<Level>,
    assets: Res<WebAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    root: Single<Entity, With<entry::WebRoot>>,
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
        &level.appearance,
    );
    if let Some(material) = materials.get_mut(&assets.blue) {
        let [r, g, b] = level.appearance.lines;
        material.base_color = Color::srgb(r, g, b);
    }
    for mut window in &mut windows {
        window.title = format!("Tempest 2032 | Level {}", level.number);
    }
}

pub(crate) fn update_lane(
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

#[cfg(test)]
#[path = "tests/scene.rs"]
mod tests;
