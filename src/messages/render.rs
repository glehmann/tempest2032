use super::{
    Messages,
    font::{Font, GlyphTriangle},
};
use bevy::{
    asset::RenderAssetUsages,
    camera::{ScalingMode, visibility::RenderLayers},
    core_pipeline::tonemapping::Tonemapping,
    mesh::PrimitiveTopology,
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
    visible: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_cells_keep_their_size_and_open_real_gaps() {
        let quad = |center: Vec2, scale| {
            [
                Vec2::new(-0.5, -0.5),
                Vec2::new(0.5, -0.5),
                Vec2::new(0.5, 0.5),
                Vec2::new(-0.5, 0.5),
            ]
            .map(|offset| cell_point(center + offset, center, scale, 0.))
        };
        for factor in [1., 1.5, 2., 4., 26.] {
            let scale = Vec2::new(1., factor);
            let a = quad(Vec2::new(0.5, 0.5), scale);
            let b = quad(Vec2::new(0.5, 1.5), scale);
            assert_eq!(a[2].y - a[1].y, 1.);
            assert_eq!(a[1].x - a[0].x, 1.);
            assert!((b[0].y - a[3].y - (factor - 1.)).abs() < 1e-6);
        }
        let p = Vec2::new(3.2, -2.6);
        let result = cell_point(p, Vec2::new(3.5, -2.5), Vec2::ONE, 0.75);
        assert!(result.distance(Vec2::new(p.x + p.y * 0.75, p.y - p.x * 0.375)) < 1e-6);
        let p = Vec2::new(4.1, -2.2);
        assert!(cell_point(p, Vec2::new(4.5, -2.5), Vec2::splat(0.5), 0.).distance(p * 0.5) < 1e-6);
    }

    #[test]
    fn recharge_geometry_is_bounded_and_cells_are_opaque() {
        let font = Font::default();
        let layout = font.layout(super::super::Id::SuperzapperRecharge);
        assert!(
            layout.len() < 40_000,
            "recharge layout grew to {} triangles",
            layout.len()
        );
        assert!(
            layout
                .iter()
                .flat_map(|glyph| glyph.colors)
                .all(|color| color.to_linear().alpha == 1.)
        );
    }

    #[test]
    fn approaching_reward_wordart_keeps_its_cells_and_area_as_it_grows() {
        let font = Font::default();
        for (id, width, height) in [
            (super::super::Id::Points2000, 197, 65),
            (super::super::Id::Excellent, 178, 42),
        ] {
            let layout = font.layout(id);
            assert!(!layout.is_empty(), "{id:?} lost its source pixels");
            assert!(
                layout.len() <= width * height * 2,
                "{id:?} cell layout grew to {} triangles",
                layout.len()
            );
            for glyph in layout {
                assert!(
                    glyph
                        .points
                        .iter()
                        .all(|p| (*p - glyph.cell).abs().max_element() <= 0.501)
                );
                let area =
                    (glyph.points[1] - glyph.points[0]).perp_dot(glyph.points[2] - glyph.points[0]);
                let points = glyph
                    .points
                    .map(|p| cell_point(p, glyph.cell, Vec2::splat(8.), 0.));
                let expanded_area = (points[1] - points[0]).perp_dot(points[2] - points[0]);
                assert!((area - expanded_area).abs() < 1e-4);
            }
        }
    }
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
    let mesh = meshes.add(message_mesh(Vec::new()));
    commands.spawn((
        Graphic,
        Mesh2d(mesh.clone()),
        MeshMaterial2d(materials.add(ColorMaterial {
            alpha_mode: AlphaMode2d::Blend,
            ..default()
        })),
        RenderLayers::layer(2),
        bevy::camera::visibility::NoFrustumCulling,
    ));
    commands.insert_resource(Assets {
        mesh,
        layouts: HashMap::new(),
        visible: false,
    });
}

pub(super) fn draw(
    messages: Res<Messages>,
    font: Res<Font>,
    mut assets: ResMut<Assets>,
    mut meshes: ResMut<bevy::asset::Assets<Mesh>>,
    cameras: Query<(&Camera, &GlobalTransform), super::geometry::GameCameras>,
) {
    let mut triangles = Vec::new();
    for active in &messages.rewards {
        if !matches!(
            active.event.id,
            super::Id::Points2000 | super::Id::Excellent
        ) {
            continue;
        }
        let id = active.event.id;
        let layout = assets.layouts.entry(id).or_insert_with(|| font.layout(id));
        let projected = active.event.origin.and_then(|origin| {
            cameras.iter().find_map(|(camera, pose)| {
                let point = camera.world_to_viewport(pose, origin).ok()?;
                let size = camera.logical_viewport_size()?;
                Some(Vec2::new(point.x - size.x / 2., size.y / 2. - point.y) * (240. / size.y))
            })
        });
        let center = if active.event.origin.is_some() {
            let Some(center) = projected else {
                continue;
            };
            center
        } else {
            Vec2::ZERO
        };
        let age = active.age as f32;
        let flash = Color::LinearRgba(
            crate::cry::color((active.age as u8).wrapping_mul(16).wrapping_add(30), 255)
                .to_linear(),
        );
        let score = matches!(
            id,
            super::Id::Points250 | super::Id::Points500 | super::Id::Points750
        );
        let cells = matches!(id, super::Id::Points2000 | super::Id::Excellent);
        let copies = if score || cells {
            1
        } else {
            (1 + active.age / 8).min(20)
        };
        for copy in (0..copies).rev() {
            let scale = if score {
                (2. + age) * 0.16
            } else {
                let depth = (210. - age - copy as f32 * age / 4.).max(1.);
                (age / 16.) * 128. / (depth + 80.)
            };
            let alpha = if score {
                (1. - (age - 50.).max(0.) / 30.).max(0.)
            } else {
                1. / (1. + copy as f32 * 0.6)
            };
            for glyph in layout.iter() {
                let centroid = glyph.points.iter().copied().sum::<Vec2>() / 3. + glyph.origin;
                let expansion = if score {
                    (age - 16.).max(0.) * 0.035
                } else {
                    0.
                };
                triangles.push(std::array::from_fn(|index| {
                    let local = if cells {
                        glyph.origin
                            + cell_point(glyph.points[index], glyph.cell, Vec2::splat(scale), 0.)
                    } else {
                        (glyph.origin + glyph.points[index]) * scale + centroid * expansion
                    };
                    let color = if id == super::Id::OneUp {
                        flash
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
    let visible = !triangles.is_empty();
    if (visible || assets.visible)
        && let Some(mesh) = meshes.get_mut(&assets.mesh)
    {
        *mesh = message_mesh(triangles);
        mesh.asset_usage = RenderAssetUsages::RENDER_WORLD;
    }
    assets.visible = visible;
}

fn cell_point(point: Vec2, center: Vec2, scale: Vec2, shear: f32) -> Vec2 {
    let local = center * scale + (point - center) * scale.min(Vec2::ONE);
    Vec2::new(local.x + local.y * shear, local.y - local.x * shear * 0.5)
}

pub(crate) fn message_mesh(triangles: Vec<[(Vec3, Color); 3]>) -> Mesh {
    let mut positions = Vec::with_capacity(triangles.len() * 3);
    let mut colors = Vec::with_capacity(triangles.len() * 3);
    for triangle in triangles {
        for (position, color) in triangle {
            positions.push(position.to_array());
            colors.push(color.to_linear().to_f32_array());
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
}
