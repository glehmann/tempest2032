//! Moving background stars and warp depth samples. Draws screen-sized dots
//! behind the web with seeded positions and original depth-cued brightness.

use crate::game::{Game, Status};
use crate::mesh::colored_mesh;
use crate::web::FRONT_RADIUS;
use crate::web::LANE_UNIT;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

pub(crate) const COUNT: usize = 180;

pub(crate) const DEPTH_WRAP: i32 = 512 * LANE_UNIT;

pub(crate) const STEP: i32 = 2 * LANE_UNIT;

pub(crate) const BACKGROUND_DISTANCE: f32 = 100.0;

#[derive(Resource, Default)]
pub(crate) struct Warp {
    pub(crate) distance: i32,
    pub(crate) spacing: i32,
    pub(crate) copies: u8,
}

pub(crate) struct Star {
    pub(crate) position: Vec2,
    pub(crate) depth: i32,
    pub(crate) color: Color,
}

impl Star {
    pub(crate) fn advance(&mut self) {
        self.depth = (self.depth - STEP).rem_euclid(DEPTH_WRAP);
    }

    pub(crate) fn projected(&self, viewpoint: Vec2) -> Vec2 {
        // llama.gas divides by (z >> 14) + 1, then shifts by seven.
        (self.position - viewpoint) * (128.0 / (self.depth as f32 / LANE_UNIT as f32 + 0.25))
    }

    pub(crate) fn color(&self) -> Color {
        let intensity = (255 - (self.depth >> 17)) as f32 / 255.0;
        let color = self.color.to_srgba();
        Color::srgb(
            color.red * intensity,
            color.green * intensity,
            color.blue * intensity,
        )
    }
}

#[derive(Resource)]
pub(crate) struct Stars(pub(crate) Vec<Star>);

#[derive(Resource)]
pub(crate) struct BonusStars(Vec<Star>);

impl Default for BonusStars {
    fn default() -> Self {
        let sines = include_bytes!("../assets/warp/sines.bin");
        let sin = |angle: usize| i32::from(sines[angle & 255] as i8);
        // rs400 runs 33 iterations per ring but the GPU reads only 256 stars.
        Self(
            (0..8)
                .rev()
                .flat_map(|ring| {
                    (0..=32).rev().map(move |step| {
                        let angle = step * 8;
                        let phase = ((angle + 64) & 255) * 4;
                        let hue = ((7 - ring) * 0x20 + (((sin(phase) + 128) & 0xf0) >> 4)) as u8;
                        Star {
                            position: Vec2::new(sin(angle) as f32, -sin(angle + 64) as f32)
                                * (400. / 512.),
                            depth: ring * 64 * LANE_UNIT + sin(phase).abs() * LANE_UNIT / 2,
                            color: crate::cry::color(hue, 255),
                        }
                    })
                })
                .take(256)
                .collect(),
        )
    }
}

impl Default for Stars {
    fn default() -> Self {
        let mut random = 0x2_000_u32;
        let mut next = || {
            random = random.wrapping_mul(1664525).wrapping_add(1013904223);
            random as f32 / u32::MAX as f32
        };
        let colors = [
            Color::srgb(0.35, 0.65, 0.75),
            Color::srgb(1.0, 0.45, 0.08),
            Color::srgb(0.8, 0.85, 0.12),
            Color::srgb(0.7, 0.7, 0.7),
        ];
        Self(
            (0..COUNT)
                .map(|index| Star {
                    position: Vec2::new((next() - 0.5) * 256.0, (next() - 0.5) * 256.0),
                    depth: (next() * DEPTH_WRAP as f32) as i32 % DEPTH_WRAP,
                    color: colors[index % colors.len()],
                })
                .collect(),
        )
    }
}

#[derive(Component)]
pub(crate) struct StarField;

pub(crate) fn setup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.insert_resource(Stars::default());
    commands.insert_resource(BonusStars::default());
    commands.insert_resource(Warp::default());
    commands.spawn((
        StarField,
        RenderLayers::layer(0),
        Mesh3d(meshes.add(colored_mesh([]))),
        MeshMaterial3d(materials.add(StandardMaterial {
            unlit: true,
            cull_mode: None,
            ..default()
        })),
        Transform::default(),
        // The mesh changes its bounds as stars cross the screen.
        bevy::camera::visibility::NoFrustumCulling,
    ));
}

pub(crate) fn add_systems(app: &mut App) {
    app.add_systems(FixedUpdate, advance_stars)
        .add_systems(Update, draw_stars.after(crate::warp::render::draw));
}

pub(crate) fn advance_stars(mut stars: ResMut<Stars>) {
    for star in &mut stars.0 {
        star.advance();
    }
}

// Project the moving stars onto a distant camera-aligned plane. This keeps the
// original's single-pixel dots while allowing the web to cover the background.
#[allow(clippy::type_complexity)]
pub(crate) fn draw_stars(
    stars: Res<Stars>,
    warp: Res<Warp>,
    game: Option<Res<Game>>,
    bonus_stars: Option<Res<BonusStars>>,
    mut meshes: ResMut<Assets<Mesh>>,
    cameras: Query<(&Transform, &Projection), (With<Camera3d>, Without<StarField>)>,
    mut fields: Query<(&Mesh3d, &mut Transform), With<StarField>>,
) {
    let Some((camera, Projection::Perspective(projection))) = cameras.iter().next() else {
        return;
    };
    let half_height = BACKGROUND_DISTANCE * (projection.fov * 0.5).tan();
    let half_width = half_height * projection.aspect_ratio;
    let pixel = 2.0 * half_width / 384.0;
    let bonus = game
        .as_ref()
        .filter(|game| game.status == Status::Bonus)
        .and_then(|game| game.bonus.as_ref());
    let bonus_warp = Warp {
        distance: bonus.map_or(0, |session| {
            ((session.tick % 512 * 3) % 512) as i32 * LANE_UNIT
        }),
        spacing: 26 * LANE_UNIT,
        copies: 5,
    };
    let warp = if bonus.is_some() { &bonus_warp } else { &*warp };
    let stars = if bonus.is_some() {
        bonus_stars.as_ref().map_or(&stars.0, |stars| &stars.0)
    } else {
        &stars.0
    };
    let viewpoint = if bonus.is_some() {
        Vec2::ZERO
    } else {
        camera.translation.truncate() * (24.0 / FRONT_RADIUS)
    };
    let tunnel = bonus.is_some_and(|session| session.kind == crate::warp::ZoneKind::Tunnel);
    let triangles = stars.iter().filter(|_| !tunnel).flat_map(|star| {
        (0..=warp.copies).flat_map(move |index| {
            let star = Star {
                position: star.position,
                depth: (star.depth - warp.distance + i32::from(index) * warp.spacing)
                    .rem_euclid(DEPTH_WRAP),
                color: star.color,
            };
            let centre = star.projected(viewpoint) * pixel;
            if centre.x.abs() > half_width || centre.y.abs() > half_height {
                return Vec::new();
            }
            let color = star.color();
            let quad = [
                Vec2::new(-0.5, -0.5),
                Vec2::new(0.5, -0.5),
                Vec2::new(0.5, 0.5),
                Vec2::new(-0.5, 0.5),
            ]
            .map(|corner| ((centre + corner * pixel).extend(0.0), color));
            vec![[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]]
        })
    });
    let geometry = colored_mesh(triangles);
    for (mesh, mut transform) in &mut fields {
        if let Some(mesh) = meshes.get_mut(&mesh.0) {
            *mesh = geometry.clone();
        }
        transform.translation =
            camera.translation + camera.rotation * Vec3::NEG_Z * BACKGROUND_DISTANCE;
        transform.rotation = camera.rotation;
    }
}

#[cfg(test)]
#[path = "tests/stars.rs"]
mod tests;
