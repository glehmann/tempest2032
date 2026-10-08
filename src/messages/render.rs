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
            triangles.extend(layout.iter().map(|glyph| {
                std::array::from_fn(|index| {
                    let p = glyph.points[index];
                    let xy = Vec2::new(
                        (glyph.origin.x + p.x * scale.x + p.y * scale.y * shear) * fit,
                        anchor
                            - (glyph.origin.y + p.y * scale.y - p.x * scale.x * shear / 2.) * fit,
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
