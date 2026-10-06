use super::*;

#[derive(Clone, Copy)]
pub(super) struct Death {
    pub(super) tick: u16,
    pub(super) visible: bool,
    camera_z: f32,
}

impl Death {
    pub(super) fn new(position: Vec3, web: &level::Web) -> Self {
        Self {
            tick: 0,
            visible: true,
            camera_z: web.camera_position(position).z,
        }
    }

    fn advance(&mut self) -> bool {
        if self.visible {
            self.tick += 1;
            self.visible = self.tick <= 120;
            self.camera_z += 17.0 / 16.0 * WORLD_UNIT;
            // zap_player calls vp_xform, which moves z back toward its target.
            if self.visible {
                self.camera_z -= 0.5 * WORLD_UNIT;
            }
        } else {
            self.camera_z += 8.0 * WORLD_UNIT;
        }
        self.camera_z >= RETREAT_END
    }
}

#[derive(Component)]
pub(super) struct ShotBurst(usize);

pub(super) fn setup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let mesh = meshes.add(zap_mesh());
    for index in 0..4 {
        commands.spawn((
            ShotBurst(index),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: cry::color(255 - index as u8 * 17, 255),
                unlit: true,
                cull_mode: None,
                ..default()
            })),
            RenderLayers::layer(1),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
}

fn zap_mesh() -> Mesh {
    let points = [
        (3., 3.),
        (8., 7.),
        (14., 3.),
        (12., 8.),
        (16., 12.),
        (11., 11.),
        (10., 16.),
        (7., 11.),
        (3., 13.),
        (6., 8.),
    ]
    .map(|(x, y)| Vec3::new((x - 9.0) * 0.075, (y - 9.0) * 0.075, 0.0));
    colored_mesh((0..points.len()).flat_map(|index| {
        let (a, b) = (points[index], points[(index + 1) % points.len()]);
        let side = (b - a).normalize().cross(Vec3::Z) * 0.012;
        let quad = [a - side, a + side, b + side, b - side].map(|point| (point, Color::WHITE));
        [[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]]
    }))
}

pub(super) fn animate(
    level: Res<Level>,
    mut game: ResMut<Game>,
    mut player: Single<&mut Transform, With<Glider>>,
    mut cameras: Query<&mut Transform, (With<Camera3d>, Without<Glider>)>,
) {
    let Some(mut death) = game.death else {
        return;
    };
    let was_visible = death.visible;
    if death.advance() {
        game.death = None;
        game.status = Status::Respawning;
        for mut camera in &mut cameras {
            camera.translation = level
                .web
                .camera_position(level.web.glider_transform(level.web.start()).translation);
        }
        return;
    }
    if was_visible {
        player.translation.z += WORLD_UNIT / 4.0;
    }
    for mut camera in &mut cameras {
        if death.visible {
            let delta = (player.translation - camera.translation)
                .clamp(Vec3::splat(-CAMERA_STEP), Vec3::splat(CAMERA_STEP));
            camera.translation.x += delta.x;
            camera.translation.y += delta.y;
        }
        camera.translation.z = death.camera_z;
    }
    game.death = Some(death);
}

pub(super) fn draw(
    game: Res<Game>,
    player: Single<&Transform, With<Glider>>,
    mut bursts: Query<(&ShotBurst, &mut Transform, &mut Visibility), Without<Glider>>,
) {
    for (burst, mut transform, mut visibility) in &mut bursts {
        *visibility = if game.death.is_some_and(|death| death.visible) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if let Some(death) = game.death {
            *transform = **player;
            transform.translation.z += burst.0 as f32 * f32::from(death.tick) * WORLD_UNIT / 2.0;
            transform.rotation *=
                Quat::from_rotation_z(f32::from((death.tick * 12) as u8) * TAU / 256.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(lives: u8) -> App {
        let mut app = App::new();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        entry::setup(
            &mut app.world_mut().commands(),
            &mut meshes,
            &mut materials,
            &Level::default().web,
        );
        setup(&mut app.world_mut().commands(), &mut meshes, &mut materials);
        app.world_mut().flush();
        app.insert_resource(Game {
            lives,
            score: 9000,
            ..default()
        })
        .insert_resource(meshes)
        .init_resource::<Level>()
        .init_resource::<Levels>()
        .insert_resource(materials)
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<powerups::Upgrades>()
        .init_resource::<ShotAssets>()
        .init_resource::<explosion::BurstAssets>()
        .add_systems(
            FixedUpdate,
            (
                move_glider.run_if(controlling),
                powerups::update_jump,
                fire_bullets.run_if(controlling),
                move_enemy_shots,
                animate,
                check_game,
                load_level,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (load_level.after(restart_game), entry::draw, draw).chain(),
        );
        app.world_mut().spawn((
            Glider {
                progress: START_PROGRESS,
                velocity: 0,
            },
            Weapon::default(),
            glider_transform(START_PROGRESS),
            Visibility::Visible,
        ));
        for _ in 0..2 {
            app.world_mut().spawn((
                Camera3d::default(),
                Transform::from_translation(camera_position(
                    glider_transform(START_PROGRESS).translation,
                )),
            ));
        }
        app.world_mut().spawn((
            EnemyShot {
                lane: selected_lane(START_PROGRESS) as usize,
                depth: 2 * LANE_UNIT + LANE_UNIT / 2,
                angle: 0,
            },
            Transform::default(),
        ));
        app
    }

    fn ticks(app: &mut App, count: usize) {
        for _ in 0..count {
            app.world_mut().run_schedule(FixedUpdate);
            app.world_mut().run_schedule(Update);
        }
    }

    #[test]
    fn shot_hit_uses_four_zap_outlines_and_locks_controls_until_the_retreat_finishes() {
        let mut app = app(3);
        app.world_mut().resource_mut::<powerups::Upgrades>().laser = true;
        app.world_mut()
            .resource_mut::<powerups::Upgrades>()
            .jump_enabled = true;
        app.world_mut().resource_mut::<powerups::Upgrades>().droid = true;
        ticks(&mut app, 1);
        assert!(app.world().resource::<Game>().status == Status::ShotHit);
        assert_eq!(app.world().resource::<Game>().lives, 3);
        assert!(!app.world().resource::<powerups::Upgrades>().laser);
        assert!(!app.world().resource::<powerups::Upgrades>().jump_enabled);
        assert!(!app.world().resource::<powerups::Upgrades>().droid);
        assert_eq!(
            app.world_mut()
                .query::<&EnemyShot>()
                .iter(app.world())
                .count(),
            0
        );
        let original = glider_transform(START_PROGRESS);
        for key in [KeyCode::KeyA, KeyCode::Enter, KeyCode::KeyJ] {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
        }
        ticks(&mut app, 10);
        let rotation = original.rotation * Quat::from_rotation_z(120.0 * TAU / 256.0);
        let mut count = 0;
        for (burst, transform, visibility, mesh, material) in app
            .world_mut()
            .query::<(
                &ShotBurst,
                &Transform,
                &Visibility,
                &Mesh3d,
                &MeshMaterial3d<StandardMaterial>,
            )>()
            .iter(app.world())
        {
            count += 1;
            assert_eq!(*visibility, Visibility::Visible);
            assert_eq!(
                transform.translation.truncate(),
                original.translation.truncate()
            );
            assert!(
                (transform.translation.z
                    - original.translation.z
                    - (2.5 + burst.0 as f32 * 5.0) * WORLD_UNIT)
                    .abs()
                    < 1e-5
            );
            assert!(transform.rotation.dot(rotation).abs() > 1.0 - 1e-6);
            assert_eq!(
                app.world()
                    .resource::<Assets<Mesh>>()
                    .get(&mesh.0)
                    .unwrap()
                    .count_vertices(),
                60
            );
            assert_eq!(
                app.world()
                    .resource::<Assets<StandardMaterial>>()
                    .get(&material.0)
                    .unwrap()
                    .base_color,
                cry::color(255 - burst.0 as u8 * 17, 255)
            );
        }
        assert_eq!(count, 4);
        assert_eq!(
            app.world_mut().query::<&Bullet>().iter(app.world()).count(),
            0
        );
        assert_eq!(
            app.world_mut()
                .query::<&Glider>()
                .single(app.world())
                .unwrap()
                .progress,
            START_PROGRESS
        );
        ticks(&mut app, 110);
        assert!(app.world().resource::<Game>().death.unwrap().visible);
        assert_eq!(app.world().resource::<Game>().lives, 3);
        for camera in app
            .world_mut()
            .query_filtered::<&Transform, With<Camera3d>>()
            .iter(app.world())
        {
            assert!((camera.translation.z - 16.55).abs() < 1e-4);
            assert!(
                camera
                    .translation
                    .truncate()
                    .distance(original.translation.truncate())
                    < 1e-5
            );
        }
        ticks(&mut app, 1);
        assert!(!app.world().resource::<Game>().death.unwrap().visible);
        assert!(
            app.world_mut()
                .query_filtered::<&Visibility, With<ShotBurst>>()
                .iter(app.world())
                .all(|visibility| *visibility == Visibility::Hidden)
        );
        ticks(&mut app, 41);
        assert_eq!(app.world().resource::<Game>().lives, 3);
        ticks(&mut app, 1);
        let game = app.world().resource::<Game>();
        assert!(game.status == Status::Entering);
        assert!(game.death.is_none());
        assert_eq!((game.lives, game.score, game.invulnerable), (2, 9000, 120));
        for camera in app
            .world_mut()
            .query_filtered::<&Transform, With<Camera3d>>()
            .iter(app.world())
        {
            assert_eq!(camera.translation, camera_position(original.translation));
        }
    }

    #[test]
    fn the_last_life_is_lost_only_after_the_full_animation() {
        let mut app = app(1);
        ticks(&mut app, 163);
        assert_eq!(app.world().resource::<Game>().lives, 1);
        ticks(&mut app, 1);
        assert!(app.world().resource::<Game>().status == Status::GameOver);
        assert_eq!(app.world().resource::<Game>().lives, 0);
        assert!(app.world().resource::<Game>().entry.is_none());
    }

    #[test]
    fn restart_cancels_the_explosion_and_restores_the_camera() {
        let mut app = app(3);
        app.add_systems(Update, restart_game.before(entry::draw).before(draw));
        ticks(&mut app, 80);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.world_mut().run_schedule(Update);
        assert!(app.world().resource::<Game>().death.is_none());
        assert!(app.world().resource::<Game>().status == Status::Entering);
        assert_eq!(app.world().resource::<Game>().lives, 3);
        assert!(
            app.world_mut()
                .query_filtered::<&Visibility, With<ShotBurst>>()
                .iter(app.world())
                .all(|visibility| *visibility == Visibility::Hidden)
        );
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

    #[test]
    fn jump_or_invulnerability_prevents_the_hit_animation() {
        for jump in [false, true] {
            let mut app = app(3);
            if jump {
                app.world_mut()
                    .resource_mut::<powerups::Upgrades>()
                    .jump_enabled = true;
                app.world_mut()
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .press(KeyCode::KeyJ);
            } else {
                app.world_mut().resource_mut::<Game>().invulnerable = 60;
            }
            ticks(&mut app, 2);
            assert!(app.world().resource::<Game>().death.is_none());
            assert_eq!(app.world().resource::<Game>().lives, 3);
        }
    }
}
