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
    fn approaching_2000_cells_keep_their_area_as_the_reward_grows() {
        let font = Font::default();
        let layout = font.layout(super::super::Id::Points2000);
        assert!(!layout.is_empty());
        assert!(
            layout.len() <= 197 * 65 * 2,
            "2000 cell layout grew to {} triangles",
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

    #[test]
    #[ignore = "manual CPU message render benchmark"]
    fn benchmark_recharge_rendering() {
        let font = Font::default();
        let layout = font.layout(super::super::Id::SuperzapperRecharge);
        eprintln!("Recharge layout: {} triangles", layout.len());
        let mut meshes = bevy::asset::Assets::<Mesh>::default();
        let mesh = meshes.add(message_mesh(Vec::new()));
        let mut app = App::new();
        app.insert_resource(font)
            .insert_resource(meshes)
            .insert_resource(Assets {
                mesh: mesh.clone(),
                layouts: HashMap::new(),
            })
            .init_resource::<crate::game::Game>()
            .insert_resource(Messages {
                banner: Some(super::super::Active {
                    event: super::super::Id::SuperzapperRecharge.into(),
                    age: 280,
                }),
                clock: 280,
                ..default()
            })
            .add_systems(Update, draw);
        app.update();
        let start = std::time::Instant::now();
        for _ in 0..200 {
            app.update();
        }
        eprintln!(
            "Message mesh: {} vertices; mean CPU draw: {:.3} ms",
            app.world()
                .resource::<bevy::asset::Assets<Mesh>>()
                .get(&mesh)
                .unwrap()
                .count_vertices(),
            start.elapsed().as_secs_f64() * 1000. / 200.
        );
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
    });
}

pub(super) fn draw(
    messages: Res<Messages>,
    font: Res<Font>,
    mut assets: ResMut<Assets>,
    mut meshes: ResMut<bevy::asset::Assets<Mesh>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    game: Res<crate::game::Game>,
) {
    let mut triangles = Vec::new();
    let viewport = windows
        .iter()
        .next()
        .map_or(384., |w| 240. * w.width() / w.height());
    if let Some(active) = messages.game_over {
        let alpha = (active.age as f32 / 60.).min(1.) * 0.8;
        let corners = [
            Vec2::new(-viewport / 2., -120.),
            Vec2::new(viewport / 2., -120.),
            Vec2::new(viewport / 2., 120.),
            Vec2::new(-viewport / 2., 120.),
        ]
        .map(|p| (p.extend(0.), Color::BLACK.with_alpha(alpha)));
        triangles.extend([
            [corners[0], corners[1], corners[2]],
            [corners[0], corners[2], corners[3]],
        ]);
    }
    if let Some(active) = messages.banner {
        let layout = assets
            .layouts
            .entry(active.event.id)
            .or_insert_with(|| font.layout(active.event.id));
        let width = font.width(active.event.id);
        let hint = matches!(active.event.id, super::Id::GreenTrack | super::Id::WarpFive);
        let small = if active.event.id == super::Id::GreenTrack {
            0.5
        } else {
            1.
        };
        let fit = ((viewport - 24.) / (width.max(1.) * small)).min(1.) * small;
        let scale = if hint { Vec2::ONE } else { active.scale() };
        let shear = if hint {
            0.
        } else {
            (((messages.clock.wrapping_mul(2) & 255) as i32 - 127).abs() - 63) as f32 / 64.
        };
        let anchor = if active.event.id == super::Id::GreenTrack {
            -60.
        } else if active.event.id == super::Id::WarpFive {
            45.
        } else {
            40.
        };
        for shadow in [true, false] {
            if shadow && scale != Vec2::ONE {
                continue;
            }
            triangles.extend(layout.iter().map(|glyph| {
                std::array::from_fn(|index| {
                    let p = glyph.points[index];
                    let local = cell_point(p, glyph.cell, scale, shear);
                    let xy = Vec2::new(
                        (glyph.origin.x + local.x) * fit,
                        anchor - (glyph.origin.y + local.y) * fit,
                    );
                    (
                        (xy + if shadow {
                            Vec2::new(0.6, -0.6)
                        } else {
                            Vec2::ZERO
                        })
                        .extend(0.),
                        if shadow {
                            Color::LinearRgba(LinearRgba::new(0., 0., 0., 0.75))
                        } else {
                            glyph.colors[index]
                        },
                    )
                })
            }));
        }
    }
    if let Some(active) = messages.game_over {
        let layout = assets
            .layouts
            .entry(super::Id::GameOver)
            .or_insert_with(|| font.layout(super::Id::GameOver));
        let sin = |angle: u32| {
            i32::from(include_bytes!("../../assets/warp/sines.bin")[(angle & 255) as usize] as i8)
                as f32
        };
        let age = active.age;
        let scale = Vec2::new(
            1. + (sin(age.wrapping_mul(15)) + 128.) / 512.,
            1. + (sin(age.wrapping_mul(17)) + 128.) / 512.,
        );
        let fit = ((viewport - 24.) / font.width(super::Id::GameOver)).min(1.);
        for copy in (0..=(active.age / 6).min(12)).rev() {
            let angle =
                (((age / 8) & 15) as f32 - 7.) * std::f32::consts::TAU / 256. * copy as f32 * 0.3;
            let rotate = Mat2::from_angle(angle);
            let opacity = if copy == 0 {
                1.
            } else {
                0.35 * 0.75_f32.powi(copy as i32)
            };
            for glyph in layout.iter() {
                triangles.push(std::array::from_fn(|index| {
                    let p =
                        rotate * (glyph.points[index] * scale * fit * (1. + copy as f32 * 0.06));
                    (
                        Vec3::new(p.x, -p.y, 0.),
                        glyph.colors[index].with_alpha(opacity),
                    )
                }));
            }
        }
        for glyph in font.triangles(&format!("FINAL SCORE\n{:08}", game.score)) {
            triangles.push(std::array::from_fn(|index| {
                let p = (glyph.origin + glyph.points[index]) * 0.55;
                (Vec3::new(p.x, -65. - p.y, 0.), glyph.colors[index])
            }));
        }
    }
    for active in &messages.rewards {
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
        let cells = id == super::Id::Points2000;
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
    if let Some(mesh) = meshes.get_mut(&assets.mesh) {
        *mesh = message_mesh(triangles);
    }
}

fn cell_point(point: Vec2, center: Vec2, scale: Vec2, shear: f32) -> Vec2 {
    let local = center * scale + (point - center) * scale.min(Vec2::ONE);
    Vec2::new(local.x + local.y * shear, local.y - local.x * shear * 0.5)
}

fn message_mesh(triangles: Vec<[(Vec3, Color); 3]>) -> Mesh {
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
