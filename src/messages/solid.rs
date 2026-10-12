//! Native-resolution 3D lettering overlay; the accepted approach effects stay 2D.

use super::{
    Id, Messages,
    geometry::{self, ADVANCE, Glyph, Glyphs, OverlayCamera, Paint, Style, Triangle},
};
use bevy::{
    camera::{ScalingMode, visibility::RenderLayers},
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
};
use std::collections::HashMap;

#[derive(Resource)]
struct Graphics {
    banner: Handle<Mesh>,
    rewards: Handle<Mesh>,
    pause: Handle<Mesh>,
    art: HashMap<Id, Glyph>,
    last: Option<FrameKey>,
}

#[derive(PartialEq)]
struct FrameKey {
    epoch: u32,
    clock: u32,
    banner: Option<(Id, u32)>,
    rewards: Vec<(Id, u32, Option<Vec3>)>,
    game_over: Option<u32>,
    viewport: f32,
    paused: bool,
    pause_choice: usize,
    views: Vec<(Mat4, Option<Vec2>)>,
}

pub(super) fn add_systems(app: &mut App) {
    app.init_resource::<Glyphs>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            draw.after(super::render::draw)
                .after(crate::game::load_level)
                .after(crate::hud::update_hud),
        );
}

fn setup(
    mut commands: Commands,
    font: Res<super::font::Font>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        OverlayCamera,
        // A few small, changing overlay meshes use direct draws.
        bevy::render::view::NoIndirectDrawing,
        Camera3d::default(),
        Camera {
            order: 3,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 240.,
            },
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(0., 0., 100.),
        RenderLayers::layer(3),
        Tonemapping::None,
        Msaa::Sample4,
    ));
    let opaque = materials.add(StandardMaterial {
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let blend = materials.add(StandardMaterial {
        unlit: true,
        cull_mode: None,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let mut spawn = |material: Handle<StandardMaterial>| {
        let mesh = meshes.add(geometry::mesh(Vec::new()));
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            RenderLayers::layer(3),
            bevy::camera::visibility::NoFrustumCulling,
        ));
        mesh
    };
    let banner = spawn(opaque.clone());
    let rewards = spawn(blend);
    let pause = spawn(opaque);
    let mut art = HashMap::new();
    for id in [Id::Points250, Id::Points500, Id::Points750, Id::OneUp] {
        let (triangles, rows, size) = font.solid_art(id);
        let mut paint = Paint::new(Style::Chrome);
        paint.rows = rows;
        paint.top = -size.y / 2.;
        paint.height = size.y;
        art.insert(
            id,
            geometry::extrude(&geometry::contours_from_triangles(&triangles), &paint),
        );
    }
    commands.insert_resource(Graphics {
        banner,
        rewards,
        pause,
        art,
        last: None,
    });
}

#[allow(clippy::too_many_arguments)]
fn draw(
    messages: Res<Messages>,
    glyphs: Res<Glyphs>,
    mut graphics: ResMut<Graphics>,
    mut meshes: ResMut<Assets<Mesh>>,
    time: Res<Time<Virtual>>,
    menu: Res<crate::pause::Menu>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), geometry::GameCameras>,
) {
    let viewport = windows
        .iter()
        .next()
        .map_or(384., |w| 240. * w.width() / w.height().max(1.));
    let key = FrameKey {
        epoch: messages.epoch,
        clock: messages.clock,
        banner: messages.banner.map(|a| (a.event.id, a.age)),
        rewards: messages
            .rewards
            .iter()
            .map(|a| (a.event.id, a.age, a.event.origin))
            .collect(),
        game_over: messages.game_over.map(|a| a.age),
        viewport,
        paused: time.is_paused(),
        pause_choice: menu.selected,
        views: cameras
            .iter()
            .map(|(c, p)| (p.to_matrix(), c.logical_viewport_size()))
            .collect(),
    };
    if graphics.last.as_ref() == Some(&key) {
        return;
    }
    let mut banner = Vec::new();
    if let Some(active) = messages.banner {
        let text = active.event.id.text();
        let width = text.lines().map(|l| l.len()).max().unwrap_or(0) as f32 * ADVANCE;
        let hint = matches!(active.event.id, Id::GreenTrack | Id::WarpFive);
        let small = if active.event.id == Id::GreenTrack {
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
        let anchor = match active.event.id {
            Id::GreenTrack => -60.,
            Id::WarpFive => 45.,
            _ => 40.,
        };
        for (line, text) in text.lines().enumerate() {
            let left = -(text.len() as f32) * ADVANCE / 2.;
            for (index, character) in text.chars().enumerate() {
                let glyph = glyphs.get(character, Style::Chrome);
                let origin = Vec2::new(left + 7.5 + index as f32 * ADVANCE, line as f32 * 20.);
                let transform = |p: Vec3, center: Vec2| {
                    let p = p - Vec3::new(7.5, 9., 0.);
                    let center = center - Vec2::new(7.5, 9.);
                    let xy = center * scale + (p.truncate() - center) * scale.min(Vec2::ONE);
                    let xy = Vec2::new(xy.x + xy.y * shear, xy.y - xy.x * shear * 0.5);
                    Vec3::new(
                        (origin.x + xy.x) * fit,
                        anchor - (origin.y + xy.y) * fit,
                        10. + p.z * fit,
                    )
                };
                if scale == Vec2::ONE {
                    banner.extend(
                        glyph
                            .triangles
                            .iter()
                            .map(|t| t.map(|(p, c)| (transform(p, Vec2::new(7.5, 9.)), c))),
                    );
                } else {
                    banner.extend(
                        glyph
                            .fragments
                            .iter()
                            .map(|f| f.triangle.map(|(p, c)| (transform(p, f.center), c))),
                    );
                }
            }
        }
    }
    let mut rewards = Vec::new();
    for active in &messages.rewards {
        if matches!(active.event.id, Id::Points2000 | Id::Excellent) {
            continue;
        }
        let Some(glyph) = graphics.art.get(&active.event.id) else {
            continue;
        };
        let center = if let Some(origin) = active.event.origin {
            let Some(p) = cameras.iter().find_map(|(camera, pose)| {
                let point = camera.world_to_viewport(pose, origin).ok()?;
                let size = camera.logical_viewport_size()?;
                Some(Vec2::new(point.x - size.x / 2., size.y / 2. - point.y) * (240. / size.y))
            }) else {
                continue;
            };
            p
        } else {
            Vec2::ZERO
        };
        let age = active.age as f32;
        let score = matches!(
            active.event.id,
            Id::Points250 | Id::Points500 | Id::Points750
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
                (age / 16.) * 128. / ((210. - age - copy as f32 * age / 4.).max(1.) + 80.)
            };
            let alpha = if score {
                (1. - (age - 50.).max(0.) / 30.).max(0.)
            } else {
                1. / (1. + copy as f32 * 0.6)
            };
            let flash =
                crate::cry::color((active.age as u8).wrapping_mul(16).wrapping_add(30), 255);
            for triangle in &glyph.triangles {
                let centroid = triangle.iter().map(|(p, _)| p.truncate()).sum::<Vec2>() / 3.;
                let expansion = if score {
                    (age - 16.).max(0.) * 0.035
                } else {
                    0.
                };
                rewards.push(triangle.map(|(p, c)| {
                    let xy = p.truncate() * scale + centroid * expansion;
                    (
                        Vec3::new(
                            center.x + xy.x,
                            center.y - xy.y,
                            12. + p.z * scale - copy as f32 * 0.01,
                        ),
                        if active.event.id == Id::OneUp {
                            flash.with_alpha(alpha)
                        } else {
                            c.with_alpha(alpha)
                        },
                    )
                }));
            }
        }
    }
    let mut paused = paused_geometry(&glyphs, viewport, time.is_paused());
    if time.is_paused() {
        let fit = ((viewport - 24.) / 174.).clamp(0.01, 1.);
        for (row, text) in crate::pause::CHOICES.iter().enumerate() {
            let selected = row == menu.selected;
            let scale = 0.65 * fit;
            let left = -(text.len() as f32 * ADVANCE - 2.) * scale / 2.;
            for (index, character) in text.chars().enumerate() {
                paused.extend(
                    glyphs
                        .get(
                            character,
                            if selected { Style::Gold } else { Style::Chrome },
                        )
                        .triangles
                        .iter()
                        .map(|triangle| {
                            triangle.map(|(p, color)| {
                                (
                                    Vec3::new(
                                        left + (index as f32 * ADVANCE + p.x) * scale,
                                        5. - row as f32 * 25. - p.y * scale,
                                        60. + p.z * scale,
                                    ),
                                    color,
                                )
                            })
                        }),
                );
            }
            if selected {
                let color = crate::cry::color(0x8f, 255);
                for side in [-1., 1.] {
                    let x = side * (text.len() as f32 * ADVANCE * scale / 2. + 10. * fit);
                    let y = 5. - row as f32 * 25. - 9. * scale;
                    paused.push([
                        (Vec3::new(x - side * 4. * fit, y, 61.), color),
                        (Vec3::new(x + side * 2. * fit, y + 4. * fit, 61.), color),
                        (Vec3::new(x + side * 2. * fit, y - 4. * fit, 61.), color),
                    ]);
                }
            }
        }
    }
    for (handle, triangles) in [
        (&graphics.banner, banner),
        (&graphics.rewards, rewards),
        (&graphics.pause, paused),
    ] {
        *meshes.get_mut(handle).unwrap() = geometry::mesh(triangles);
    }
    graphics.last = Some(key);
}

fn paused_geometry(glyphs: &Glyphs, viewport: f32, paused: bool) -> Vec<Triangle> {
    if !paused {
        return Vec::new();
    }
    let fit = ((viewport - 24.) / 174.).clamp(0.01, 1.);
    let mut triangles = Vec::new();
    for (index, character) in "PAUSED".chars().enumerate() {
        let offset = -87. + index as f32 * 29.;
        triangles.extend(
            glyphs
                .get(character, Style::Gold)
                .triangles
                .iter()
                .map(|t| {
                    t.map(|(p, c)| {
                        (
                            Vec3::new(
                                (offset + p.x * 27. / 15.) * fit,
                                55. - p.y * 31. / 18. * fit,
                                60. + p.z * fit,
                            ),
                            c,
                        )
                    })
                }),
        );
    }
    triangles
}

#[cfg(test)]
#[path = "../tests/solid.rs"]
mod tests;
