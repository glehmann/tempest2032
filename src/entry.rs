use super::*;

const APPROACH_TICKS: u16 = 256;
const REZ_TICKS: u16 = 64;

#[derive(Component)]
pub(crate) struct WebRoot;

#[derive(Component)]
pub(super) struct ClawTrail(usize);

#[derive(Resource)]
pub(super) struct EntryAssets {
    frames: [Handle<Mesh>; 8],
    surface: Handle<StandardMaterial>,
}

pub(crate) fn setup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> (Entity, Handle<StandardMaterial>) {
    let frames = std::array::from_fn(|frame| meshes.add(wire_claw_mesh(frame)));
    for index in 0..4 {
        commands.spawn((
            ClawTrail(index),
            Mesh3d(frames[0].clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: cry::color(255 - index as u8, 255),
                unlit: true,
                cull_mode: None,
                ..default()
            })),
            RenderLayers::layer(1),
            Visibility::Hidden,
            Transform::default(),
        ));
    }
    let surface = materials.add(StandardMaterial {
        base_color: Color::WHITE.with_alpha(0.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    commands.insert_resource(EntryAssets {
        frames,
        surface: surface.clone(),
    });
    (
        commands
            .spawn((WebRoot, web_transform(0), Visibility::Visible))
            .id(),
        surface,
    )
}

fn web_transform(tick: u16) -> Transform {
    if tick >= APPROACH_TICKS {
        return Transform::default();
    }
    let unit = TAU / 256.0;
    let rotation = if tick == 0 {
        Quat::IDENTITY
    } else {
        Quat::from_rotation_x((128 + tick / 2) as f32 * unit)
            * Quat::from_rotation_y(tick as f32 * unit)
    };
    let center = Vec3::NEG_Z * (DEPTH * 0.5);
    Transform::from_translation(
        center - rotation * center - Vec3::Z * f32::from(APPROACH_TICKS - tick) * WORLD_UNIT,
    )
    .with_rotation(rotation)
}

fn vector_pose(progress: i32) -> (usize, f32) {
    let phase = (-(progress + LANE_UNIT / 2)).rem_euclid(LANE_UNIT);
    let frame = ((phase >> 12) & 7) as usize;
    if phase & 0x8000 == 0 {
        (7 - frame, -1.0)
    } else {
        (frame, 1.0)
    }
}

fn wire_claw_mesh(frame: usize) -> Mesh {
    // claw1 through claw8, including their closed eight-edge connection list.
    let vertices = [
        [
            (6., 4.),
            (3., 9.),
            (9., 12.),
            (15., 9.),
            (12., 4.),
            (17., 9.),
            (9., 15.),
            (1., 9.),
        ],
        [
            (5., 4.),
            (3., 10.),
            (10., 12.),
            (15., 9.),
            (12., 4.),
            (17., 9.),
            (10., 15.),
            (1., 10.),
        ],
        [
            (4., 5.),
            (4., 11.),
            (11., 12.),
            (15., 8.),
            (12., 4.),
            (17., 8.),
            (11., 15.),
            (2., 12.),
        ],
        [
            (4., 6.),
            (5., 12.),
            (11., 13.),
            (15., 8.),
            (11., 3.),
            (17., 7.),
            (12., 16.),
            (3., 13.),
        ],
        [
            (3., 8.),
            (6., 13.),
            (12., 13.),
            (15., 7.),
            (11., 3.),
            (17., 6.),
            (13., 16.),
            (4., 14.),
        ],
        [
            (3., 9.),
            (7., 13.),
            (13., 13.),
            (15., 6.),
            (10., 3.),
            (17., 5.),
            (15., 16.),
            (6., 15.),
        ],
        [
            (4., 11.),
            (10., 14.),
            (14., 13.),
            (15., 6.),
            (10., 3.),
            (17., 4.),
            (16., 15.),
            (10., 16.),
        ],
        [
            (5., 14.),
            (11., 14.),
            (14., 12.),
            (15., 6.),
            (9., 3.),
            (16., 4.),
            (17., 13.),
            (11., 16.),
        ],
    ][frame]
        .map(|(x, y)| Vec3::new((x - 9.0) * 0.075, (y - 9.0) * 0.075, 0.0));
    colored_mesh((0..8).flat_map(|index| {
        let (a, b) = (vertices[index], vertices[(index + 1) % 8]);
        let side = (b - a).normalize().cross(Vec3::Z) * 0.012;
        let quad = [a - side, a + side, b + side, b - side].map(|p| (p, Color::WHITE));
        [[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]]
    }))
}

pub(super) fn advance(mut game: ResMut<Game>) {
    let Some(tick) = game.entry else {
        return;
    };
    if !matches!(game.status, Status::Entering | Status::Playing) {
        return;
    }
    let tick = tick + 1;
    if tick == APPROACH_TICKS {
        game.status = Status::Playing;
    }
    game.entry = (tick < APPROACH_TICKS + REZ_TICKS).then_some(tick);
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn draw(
    game: Res<Game>,
    assets: Res<EntryAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut web: Single<(&mut Transform, &mut Visibility), With<WebRoot>>,
    mut player: Single<(&Glider, &Transform, &mut Visibility), Without<WebRoot>>,
    mut trails: Query<
        (&ClawTrail, &mut Transform, &mut Mesh3d, &mut Visibility),
        (Without<Glider>, Without<WebRoot>),
    >,
    mut highlights: Query<
        &mut Visibility,
        (
            With<LaneHighlight>,
            Without<Glider>,
            Without<ClawTrail>,
            Without<WebRoot>,
        ),
    >,
    mut droids: Query<
        &mut Visibility,
        (
            With<powerups::Droid>,
            Without<Glider>,
            Without<ClawTrail>,
            Without<WebRoot>,
            Without<LaneHighlight>,
        ),
    >,
) {
    let tick = game.entry.unwrap_or(APPROACH_TICKS + REZ_TICKS);
    let (web_transform, web_visibility) = &mut *web;
    **web_transform = self::web_transform(tick);
    **web_visibility = if game
        .exit
        .is_some_and(|exit| !exit.web_visible || exit.flash > 0)
    {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    if let Some(surface) = materials.get_mut(&assets.surface) {
        let alpha = game.exit.map_or_else(
            || (f32::from(tick.saturating_sub(APPROACH_TICKS)) * 8.0 / 248.0).min(1.0),
            |exit| (1.0 - f32::from(exit.ticks) * 8.0 / 248.0).max(0.0),
        );
        surface.base_color = Color::WHITE.with_alpha(game.death.map_or(alpha, |death| {
            (1.0 - f32::from(death.tick) * 8.0 / 248.0).max(0.0)
        }));
    }
    let (glider, transform, visibility) = &mut *player;
    if game.entry.is_some() || game.exit.is_some_and(|exit| exit.wireframe) || game.death.is_some()
    {
        **visibility = Visibility::Hidden;
    }
    // check_game owns the invulnerability blink once the claw has rezzed.
    else if game.invulnerable == 0 {
        **visibility = Visibility::Visible;
    }
    for mut visibility in &mut highlights {
        *visibility = if tick < APPROACH_TICKS
            || game.exit.is_some_and(|exit| exit.wireframe)
            || game.death.is_some()
        {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
    for mut visibility in &mut droids {
        *visibility = if tick < APPROACH_TICKS || game.exit.is_some() || game.death.is_some() {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
    let (frame, mirror) = vector_pose(glider.progress);
    for (trail, mut copy, mut mesh, mut visibility) in &mut trails {
        let exiting = game
            .exit
            .is_some_and(|exit| exit.wireframe && exit.flash == 0);
        *visibility = if game.death.is_none()
            && (exiting || (APPROACH_TICKS + 1..APPROACH_TICKS + REZ_TICKS).contains(&tick))
        {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        *copy = **transform;
        let spacing = game
            .exit
            .map_or(f32::from(APPROACH_TICKS + REZ_TICKS - tick) / 4.0, |exit| {
                exit.spacing as f32 / LANE_UNIT as f32
            });
        copy.translation.z += trail.0 as f32 * spacing * WORLD_UNIT;
        if let Some(exit) = game.exit {
            copy.rotation *= Quat::from_rotation_z(f32::from(exit.spin) * TAU / 256.0);
        }
        copy.scale.x = mirror;
        mesh.0 = assets.frames[frame].clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        setup(&mut app.world_mut().commands(), &mut meshes, &mut materials);
        app.world_mut().flush();
        let mut keys = ButtonInput::default();
        keys.press(KeyCode::KeyA);
        keys.press(KeyCode::Enter);
        app.insert_resource(Game {
            status: Status::Entering,
            entry: Some(0),
            ..default()
        })
        .insert_resource(keys)
        .insert_resource(meshes)
        .insert_resource(materials)
        .init_resource::<powerups::Upgrades>()
        .init_resource::<ShotAssets>()
        .init_resource::<FlipperAssets>()
        .add_systems(
            FixedUpdate,
            (
                move_glider.run_if(playing),
                fire_bullets.run_if(playing),
                spawn_enemies,
                advance,
            )
                .chain(),
        )
        .add_systems(Update, draw);
        app.world_mut().spawn((
            Glider {
                progress: START_PROGRESS,
                velocity: 0,
            },
            Weapon::default(),
            glider_transform(START_PROGRESS),
            Visibility::Hidden,
        ));
        for _ in 0..2 {
            app.world_mut().spawn((
                Camera3d::default(),
                Transform::from_translation(camera_position(
                    glider_transform(START_PROGRESS).translation,
                )),
            ));
        }
        app.world_mut().run_schedule(Update);
        app
    }

    fn ticks(app: &mut App, count: usize) {
        for _ in 0..count {
            app.world_mut().run_schedule(FixedUpdate);
            app.world_mut().run_schedule(Update);
        }
    }

    #[test]
    fn web_approaches_and_rotates_about_its_center_without_moving_the_camera() {
        let mut app = app();
        let center = Vec3::NEG_Z * (DEPTH * 0.5);
        for tick in [0u16, 64, 128, 192, 256] {
            if tick > 0 {
                ticks(&mut app, 64);
            }
            let web = app
                .world_mut()
                .query_filtered::<&Transform, With<WebRoot>>()
                .single(app.world())
                .unwrap();
            assert!(
                (web.translation + web.rotation * center)
                    .distance(center - Vec3::Z * f32::from(256 - tick) * WORLD_UNIT)
                    < 1e-4
            );
            if tick == 128 {
                let expected =
                    Quat::from_rotation_x(3.0 * TAU / 4.0) * Quat::from_rotation_y(TAU / 2.0);
                assert!(web.rotation.dot(expected).abs() > 1.0 - 1e-6);
            }
            for camera in app
                .world_mut()
                .query_filtered::<&Transform, With<Camera3d>>()
                .iter(app.world())
            {
                assert_eq!(
                    camera.translation,
                    camera_position(glider_transform(START_PROGRESS).translation)
                );
            }
        }
        let web = app
            .world_mut()
            .query_filtered::<&Transform, With<WebRoot>>()
            .single(app.world())
            .unwrap();
        assert_eq!(*web, Transform::default());
        assert_eq!(app.world().resource::<Game>().remaining, 25);
        assert_eq!(
            app.world_mut().query::<&Bullet>().iter(app.world()).count(),
            0
        );
    }

    #[test]
    fn four_wire_claws_contract_while_controls_and_the_wave_resume() {
        let mut app = app();
        ticks(&mut app, 256);
        assert!(app.world().resource::<Game>().status == Status::Playing);
        assert_eq!(app.world().resource::<Game>().spawn_timer, 60);
        ticks(&mut app, 1);
        let (glider, transform, visibility) = app
            .world_mut()
            .query::<(&Glider, &Transform, &Visibility)>()
            .single(app.world())
            .unwrap();
        assert_eq!(glider.velocity, ACCELERATION);
        assert_eq!(*visibility, Visibility::Hidden);
        let position = transform.translation;
        for (trail, transform, visibility) in app
            .world_mut()
            .query::<(&ClawTrail, &Transform, &Visibility)>()
            .iter(app.world())
        {
            assert_eq!(*visibility, Visibility::Visible);
            assert!(
                (transform.translation.z - position.z - trail.0 as f32 * 15.75 * WORLD_UNIT).abs()
                    < 1e-5
            );
        }
        assert_eq!(
            app.world_mut().query::<&Bullet>().iter(app.world()).count(),
            1
        );
        ticks(&mut app, 30);
        let assets = app.world().resource::<EntryAssets>();
        assert_eq!(
            app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(&assets.surface)
                .unwrap()
                .base_color
                .alpha(),
            1.0
        );
        ticks(&mut app, 30);
        assert_eq!(app.world().resource::<Game>().remaining, 24);
        ticks(&mut app, 3);
        assert!(app.world().resource::<Game>().entry.is_none());
        assert_eq!(
            *app.world_mut()
                .query_filtered::<&Visibility, With<Glider>>()
                .single(app.world())
                .unwrap(),
            Visibility::Visible
        );
        assert!(
            app.world_mut()
                .query_filtered::<&Visibility, With<ClawTrail>>()
                .iter(app.world())
                .all(|visibility| *visibility == Visibility::Hidden)
        );
    }

    #[test]
    fn restart_rewinds_the_approach_and_hides_the_claw() {
        let mut app = app();
        app.add_systems(Update, restart_game.before(draw));
        ticks(&mut app, 300);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.world_mut().run_schedule(Update);
        assert!(app.world().resource::<Game>().status == Status::Entering);
        assert_eq!(app.world().resource::<Game>().entry, Some(0));
        assert_eq!(
            *app.world_mut()
                .query_filtered::<&Transform, With<WebRoot>>()
                .single(app.world())
                .unwrap(),
            web_transform(0)
        );
        assert_eq!(
            *app.world_mut()
                .query_filtered::<&Visibility, With<Glider>>()
                .single(app.world())
                .unwrap(),
            Visibility::Hidden
        );
        assert!(
            app.world_mut()
                .query_filtered::<&Visibility, With<ClawTrail>>()
                .iter(app.world())
                .all(|visibility| *visibility == Visibility::Hidden)
        );
    }
}
