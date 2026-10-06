use super::*;

const COUNT: usize = 180;
const DEPTH_WRAP: i32 = 512 * LANE_UNIT;
const STEP: i32 = 2 * LANE_UNIT;
const BACKGROUND_DISTANCE: f32 = 100.0;

#[derive(Resource, Default)]
pub(crate) struct Warp {
    pub(crate) distance: i32,
    pub(crate) spacing: i32,
    pub(crate) copies: u8,
}

struct Star {
    position: Vec2,
    depth: i32,
    color: Color,
}

impl Star {
    fn advance(&mut self) {
        self.depth = (self.depth - STEP).rem_euclid(DEPTH_WRAP);
    }

    fn projected(&self, viewpoint: Vec2) -> Vec2 {
        // llama.gas divides by (z >> 14) + 1, then shifts by seven.
        (self.position - viewpoint) * (128.0 / (self.depth as f32 / LANE_UNIT as f32 + 0.25))
    }

    fn color(&self) -> Color {
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
struct Stars(Vec<Star>);

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
struct StarField;

pub(super) fn setup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.insert_resource(Stars::default());
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

pub(super) fn add_systems(app: &mut App) {
    app.add_systems(FixedUpdate, advance_stars)
        .add_systems(Update, draw_stars);
}

fn advance_stars(mut stars: ResMut<Stars>) {
    for star in &mut stars.0 {
        star.advance();
    }
}

// Project the moving stars onto a distant camera-aligned plane. This keeps the
// original's single-pixel dots while allowing the web to cover the background.
#[allow(clippy::type_complexity)]
fn draw_stars(
    stars: Res<Stars>,
    warp: Res<Warp>,
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
    let warp = &*warp;
    let triangles = stars.0.iter().flat_map(|star| {
        (0..=warp.copies).flat_map(move |index| {
            let star = Star {
                position: star.position,
                depth: (star.depth - warp.distance + i32::from(index) * warp.spacing)
                    .rem_euclid(DEPTH_WRAP),
                color: star.color,
            };
            let centre =
                star.projected(camera.translation.truncate() * (24.0 / FRONT_RADIUS)) * pixel;
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
mod tests {
    use super::*;

    #[test]
    fn warp_draws_four_depth_spaced_samples_and_returns_to_single_dots() {
        let mut app = App::new();
        let mut meshes = Assets::<Mesh>::default();
        let mesh = meshes.add(colored_mesh([]));
        app.insert_resource(meshes)
            .insert_resource(Stars(vec![Star {
                position: Vec2::ZERO,
                depth: 32 * LANE_UNIT,
                color: Color::WHITE,
            }]))
            .insert_resource(Warp {
                distance: 0,
                spacing: 16 * LANE_UNIT,
                copies: 3,
            })
            .add_systems(Update, draw_stars);
        app.world_mut().spawn((
            Camera3d::default(),
            Transform::default(),
            Projection::Perspective(PerspectiveProjection::default()),
        ));
        app.world_mut()
            .spawn((StarField, Mesh3d(mesh.clone()), Transform::default()));
        app.world_mut().run_schedule(Update);
        let colors = match app
            .world()
            .resource::<Assets<Mesh>>()
            .get(&mesh)
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_COLOR)
            .unwrap()
        {
            bevy::mesh::VertexAttributeValues::Float32x4(colors) => colors,
            _ => panic!("star mesh needs colors"),
        };
        assert_eq!(colors.len(), 24);
        assert!(
            colors
                .chunks_exact(6)
                .map(|dot| dot[0][0])
                .collect::<Vec<_>>()
                .windows(2)
                .all(|pair| pair[0] > pair[1])
        );
        *app.world_mut().resource_mut::<Warp>() = Warp::default();
        app.world_mut().run_schedule(Update);
        assert_eq!(
            app.world()
                .resource::<Assets<Mesh>>()
                .get(&mesh)
                .unwrap()
                .count_vertices(),
            6
        );
    }

    fn star(depth: i32) -> Star {
        Star {
            position: Vec2::new(30.0, -20.0),
            depth,
            color: Color::WHITE,
        }
    }

    #[test]
    fn stars_accelerate_outward_and_brighten_as_the_viewpoint_advances() {
        let mut star = star(100 * LANE_UNIT);
        let before = star.projected(Vec2::ZERO);
        let brightness = star.color().to_srgba().red;
        star.advance();
        let first_step = star.projected(Vec2::ZERO) - before;
        assert!(star.projected(Vec2::ZERO).length() > before.length());
        assert!(star.color().to_srgba().red > brightness);
        let before = star.projected(Vec2::ZERO);
        star.advance();
        assert!((star.projected(Vec2::ZERO) - before).length() > first_step.length());
    }

    #[test]
    fn near_stars_wrap_back_into_the_distance_without_changing_their_ray() {
        let mut star = star(LANE_UNIT);
        let direction = star.projected(Vec2::ZERO).normalize();
        star.advance();
        assert_eq!(star.depth, 511 * LANE_UNIT);
        assert!(star.projected(Vec2::ZERO).normalize().distance(direction) < 1e-6);
        assert!(star.projected(Vec2::ZERO).is_finite());
    }

    #[test]
    fn motion_repeats_after_256_ticks_without_changing_the_population() {
        let mut stars = Stars::default();
        let initial: Vec<_> = stars.0.iter().map(|star| star.depth).collect();
        for _ in 0..256 {
            for star in &mut stars.0 {
                star.advance();
            }
        }
        assert_eq!(stars.0.len(), COUNT);
        assert_eq!(
            stars.0.iter().map(|star| star.depth).collect::<Vec<_>>(),
            initial
        );
    }
}
