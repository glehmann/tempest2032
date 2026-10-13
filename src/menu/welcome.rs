//! The welcome table shares the HUD's beveled glyphs in a native-resolution UI viewport.

use super::{FrontEnd, Screen};
use crate::{
    difficulty::Difficulty,
    messages::geometry::{self, ADVANCE, Glyphs, OverlayCamera, Style},
    scores::{Entry, Store},
};
use bevy::{
    camera::{RenderTarget, ScalingMode, visibility::RenderLayers},
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
    render::render_resource::TextureFormat,
};

#[derive(Component)]
pub(super) struct Scores;

#[derive(Component)]
struct TableCamera;

#[derive(Resource)]
struct Table {
    mesh: Handle<Mesh>,
    previous: Option<(Difficulty, Vec<Entry>, Option<usize>)>,
}

pub(super) fn add_systems(app: &mut App) {
    app.add_systems(Update, draw.after(super::draw));
}

pub(super) fn setup(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Entity {
    let target = images.add(Image::new_target_texture(
        1,
        1,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    let camera = commands
        .spawn((
            TableCamera,
            OverlayCamera,
            Camera3d::default(),
            Camera {
                order: -1,
                clear_color: ClearColorConfig::Custom(Color::NONE),
                ..default()
            },
            RenderTarget::Image(target.into()),
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::AutoMin {
                    min_width: 560.,
                    min_height: 390.,
                },
                ..OrthographicProjection::default_3d()
            }),
            Transform::from_xyz(0., 0., 100.),
            RenderLayers::layer(4),
            bevy::render::view::NoIndirectDrawing,
            Tonemapping::None,
            Msaa::Sample4,
        ))
        .id();
    let mesh = meshes.add(geometry::mesh(Vec::new()));
    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            unlit: true,
            cull_mode: None,
            ..default()
        })),
        RenderLayers::layer(4),
        bevy::camera::visibility::NoFrustumCulling,
    ));
    commands.insert_resource(Table {
        mesh,
        previous: None,
    });
    camera
}

fn draw(
    front: Res<FrontEnd>,
    store: Res<Store>,
    difficulty: Res<Difficulty>,
    glyphs: Res<Glyphs>,
    mut table: ResMut<Table>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut camera: Single<&mut Camera, With<TableCamera>>,
) {
    camera.is_active = front.screen == Screen::Welcome;
    if !camera.is_active {
        return;
    }
    let entries = &store.data.tables[difficulty.index()];
    let highlight = front
        .highlight
        .filter(|(level, _)| *level == *difficulty)
        .map(|(_, rank)| rank);
    if table
        .previous
        .as_ref()
        .is_some_and(|(level, scores, marked)| {
            *level == *difficulty && scores == entries && *marked == highlight
        })
    {
        return;
    }
    let mut triangles = Vec::new();
    let mut text = |value: &str, x: f32, y: f32, style: Style| {
        for (index, character) in value.chars().enumerate() {
            triangles.extend(
                glyphs
                    .get(character, style)
                    .triangles
                    .iter()
                    .map(|triangle| {
                        triangle.map(|(p, color)| {
                            (
                                Vec3::new(
                                    x + (p.x + index as f32 * ADVANCE) * 1.2,
                                    y - p.y * 1.2,
                                    p.z * 1.2,
                                ),
                                color,
                            )
                        })
                    }),
            );
        }
    };
    text("HIGH SCORES", -11. * ADVANCE * 0.6, 181., Style::Chrome);
    text(
        difficulty.label(),
        -(difficulty.label().len() as f32) * ADVANCE * 0.6,
        148.,
        Style::Chrome,
    );
    for rank in 0..crate::scores::LIMIT {
        let y = 102. - rank as f32 * 29.;
        text(&format!("{:2}", rank + 1), -235., y, Style::Chrome);
        let entry = entries.get(rank);
        text(
            entry.map_or("---", |entry| entry.initials.as_str()),
            -125.,
            y,
            Style::Chrome,
        );
        let score = entry.map_or_else(|| "--------".into(), |entry| entry.score.to_string());
        text(
            &score,
            260. - score.len() as f32 * ADVANCE * 1.2,
            y,
            Style::Rainbow,
        );
    }
    if let Some(rank) = highlight {
        let y = 91. - rank as f32 * 29.;
        triangles.push([
            (Vec3::new(-269., y + 8., 1.), Color::srgb(1., 0.88, 0.48)),
            (Vec3::new(-258., y, 1.), Color::srgb(1., 0.88, 0.48)),
            (Vec3::new(-269., y - 8., 1.), Color::srgb(1., 0.88, 0.48)),
        ]);
    }
    *meshes.get_mut(&table.mesh).unwrap() = geometry::mesh(triangles);
    table.previous = Some((*difficulty, entries.clone(), highlight));
}
