//! Native-resolution game lettering for Welcome and high-score initials entry.

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
pub(super) struct Canvas;

#[derive(Component)]
struct PanelCamera;

#[derive(Resource)]
struct Graphics {
    mesh: Handle<Mesh>,
    previous: Option<Frame>,
}

#[derive(PartialEq)]
enum Frame {
    Welcome(Difficulty, Vec<Entry>, Option<usize>),
    Initials(Difficulty, u32, [u8; 3], usize),
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
            PanelCamera,
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
    commands.insert_resource(Graphics {
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
    mut graphics: ResMut<Graphics>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut camera: Single<&mut Camera, With<PanelCamera>>,
) {
    camera.is_active = matches!(front.screen, Screen::Welcome | Screen::Initials);
    if !camera.is_active {
        return;
    }
    let entries = &store.data.tables[difficulty.index()];
    let highlight = front
        .highlight
        .filter(|(level, _)| *level == *difficulty)
        .map(|(_, rank)| rank);
    let frame = if front.screen == Screen::Welcome {
        if matches!(&graphics.previous, Some(Frame::Welcome(level, scores, marked))
            if *level == *difficulty && scores == entries && *marked == highlight)
        {
            return;
        }
        Frame::Welcome(*difficulty, entries.clone(), highlight)
    } else {
        let frame = Frame::Initials(*difficulty, front.score, front.initials, front.cursor);
        if graphics.previous.as_ref() == Some(&frame) {
            return;
        }
        frame
    };
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
    if front.screen == Screen::Welcome {
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
    } else {
        text("NEW HIGH SCORE", -14. * ADVANCE * 0.6, 181., Style::Chrome);
        text(&front.score.to_string(), -235., 148., Style::Rainbow);
        text(
            difficulty.label(),
            235. - difficulty.label().len() as f32 * ADVANCE * 1.2,
            148.,
            Style::Chrome,
        );
        text(
            "ENTER YOUR INITIALS",
            -19. * ADVANCE * 0.6,
            116.,
            Style::Chrome,
        );
        // Keep the selected initials above the Steam Deck keyboard's lower half.
        for (index, letter) in front.initials.iter().enumerate() {
            text(
                &char::from(*letter).to_string(),
                index as f32 * 80. - 89.,
                85.,
                if index == front.cursor {
                    Style::Rainbow
                } else {
                    Style::Chrome
                },
            );
        }
        let x = front.cursor as f32 * 80. - 80.;
        let corners = [
            Vec3::new(x - 15., 61., 1.),
            Vec3::new(x + 15., 61., 1.),
            Vec3::new(x + 15., 58., 1.),
            Vec3::new(x - 15., 58., 1.),
        ];
        triangles.extend(
            [[0, 1, 2], [0, 2, 3]]
                .map(|face| face.map(|index| (corners[index], Color::srgb(1., 0.88, 0.48)))),
        );
    }
    *meshes.get_mut(&graphics.mesh).unwrap() = geometry::mesh(triangles);
    graphics.previous = Some(frame);
}
