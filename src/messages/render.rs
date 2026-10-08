use super::{
    Messages,
    font::{Font, GlyphTriangle},
};
use crate::mesh::colored_mesh;
use bevy::{
    camera::{ScalingMode, visibility::RenderLayers},
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
    sprite_render::AlphaMode2d,
};
use std::collections::HashMap;

#[derive(Component)]
struct Graphic;

#[derive(Resource)]
pub(super) struct Assets {
    mesh: Handle<Mesh>,
    layouts: HashMap<super::Id, Vec<GlyphTriangle>>,
}

pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<bevy::asset::Assets<Mesh>>,
    mut materials: ResMut<bevy::asset::Assets<ColorMaterial>>,
) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 2,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 240.,
            },
            ..OrthographicProjection::default_2d()
        }),
        RenderLayers::layer(2),
        Tonemapping::None,
        Msaa::Sample4,
    ));
    let mesh = meshes.add(colored_mesh([]));
    commands.spawn((
        Graphic,
        Mesh2d(mesh.clone()),
        MeshMaterial2d(materials.add(ColorMaterial {
            alpha_mode: AlphaMode2d::Blend,
            ..default()
        })),
        RenderLayers::layer(2),
    ));
    commands.insert_resource(Assets {
        mesh,
        layouts: HashMap::new(),
    });
}

pub(super) fn draw(
    messages: Res<Messages>,
    font: Res<Font>,
    mut assets: ResMut<Assets>,
    mut meshes: ResMut<bevy::asset::Assets<Mesh>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    let mut triangles = Vec::new();
    if let Some(active) = messages.banner {
        let layout = assets
            .layouts
            .entry(active.event.id)
            .or_insert_with(|| font.layout(active.event.id));
        let width = font.width(active.event.id);
        let viewport = windows
            .iter()
            .next()
            .map_or(384., |w| 240. * w.width() / w.height());
        let fit = ((viewport - 24.) / width.max(1.)).min(1.);
        let scale = active.scale();
        let shear = (((messages.clock.wrapping_mul(2) & 255) as i32 - 127).abs() - 63) as f32 / 64.;
        for shadow in [true, false] {
            triangles.extend(layout.iter().map(|glyph| {
                std::array::from_fn(|index| {
                    let p = glyph.points[index];
                    let xy = Vec2::new(
                        (glyph.origin.x + p.x * scale.x + p.y * scale.y * shear) * fit,
                        40. - (glyph.origin.y + p.y * scale.y - p.x * scale.x * shear / 2.) * fit,
                    );
                    (
                        (xy + if shadow {
                            Vec2::new(0.6, -0.6)
                        } else {
                            Vec2::ZERO
                        })
                        .extend(0.),
                        if shadow {
                            Color::BLACK.with_alpha(0.75)
                        } else {
                            glyph.colors[index]
                        },
                    )
                })
            }));
        }
    }
    for active in &messages.rewards {
        let id = active.event.id;
        let layout = assets.layouts.entry(id).or_insert_with(|| font.layout(id));
        let center = active
            .event
            .origin
            .and_then(|origin| {
                cameras.iter().find_map(|(camera, pose)| {
                    let point = camera.world_to_viewport(pose, origin).ok()?;
                    let size = camera.logical_viewport_size()?;
                    Some(Vec2::new(point.x - size.x / 2., size.y / 2. - point.y) * (240. / size.y))
                })
            })
            .unwrap_or(Vec2::ZERO);
        let age = f32::from(active.age);
        let score = matches!(
            id,
            super::Id::Points250 | super::Id::Points500 | super::Id::Points750
        );
        let copies = if score {
            1
        } else {
            (1 + active.age / 8).min(20)
        };
        for copy in (0..copies).rev() {
            let scale = if score {
                (2. + age) * 0.16
            } else {
                let depth = (210. - age - f32::from(copy) * age / 4.).max(1.);
                (age / 16.) * 128. / (depth + 80.)
            };
            let alpha = if score {
                (1. - (age - 50.).max(0.) / 30.).max(0.)
            } else {
                1. / (1. + f32::from(copy) * 0.6)
            };
            for glyph in layout.iter() {
                let centroid = glyph.points.iter().copied().sum::<Vec2>() / 3. + glyph.origin;
                let expansion = if score {
                    (age - 16.).max(0.) * 0.035
                } else {
                    0.
                };
                triangles.push(std::array::from_fn(|index| {
                    let local = (glyph.origin + glyph.points[index]) * scale + centroid * expansion;
                    let color = if id == super::Id::OneUp {
                        crate::cry::color((active.age as u8).wrapping_mul(16).wrapping_add(30), 255)
                    } else {
                        glyph.colors[index]
                    };
                    (
                        (center + Vec2::new(local.x, -local.y)).extend(0.),
                        color.with_alpha(alpha),
                    )
                }));
            }
        }
    }
    if let Some(mesh) = meshes.get_mut(&assets.mesh) {
        *mesh = colored_mesh(triangles);
    }
}
