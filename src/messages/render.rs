use super::{
    Messages,
    font::{self, Font, GlyphTriangle},
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
) {
    let mut triangles = Vec::new();
    if let Some(active) = messages.banner {
        let text = active.event.id.text();
        let layout = assets
            .layouts
            .entry(active.event.id)
            .or_insert_with(|| font.triangles(text));
        let width = text.lines().map(|line| line.len()).max().unwrap_or(0) as f32 * 17.;
        let viewport = windows
            .iter()
            .next()
            .map_or(384., |w| 240. * w.width() / w.height());
        let fit = ((viewport - 24.) / width.max(1.)).min(1.);
        let scale = active.scale();
        let shear = (((messages.clock.wrapping_mul(2) & 255) as i32 - 127).abs() - 63) as f32 / 64.;
        for shadow in [true, false] {
            triangles.extend(layout.iter().map(|glyph| {
                glyph.points.map(|p| {
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
                            font::color(p.y + 9.)
                        },
                    )
                })
            }));
        }
    }
    if let Some(mesh) = meshes.get_mut(&assets.mesh) {
        *mesh = colored_mesh(triangles);
    }
}
