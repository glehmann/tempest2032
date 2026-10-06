use super::*;

const ZOOM_ACCELERATION: i32 = 0x400;
const FLASH_TICKS: u8 = 16;

#[derive(Clone, Copy)]
pub(super) struct Exit {
    speed: i32,
    extra_depth: i32,
    camera_z: f32,
    rear: i32,
    web_end: f32,
    warp_end: i32,
    pub(super) distance: i32,
    pub(super) depth: i32,
    pub(super) ticks: u16,
    pub(super) spin: u8,
    pub(super) spacing: i32,
    pub(super) wireframe: bool,
    pub(super) web_visible: bool,
    pub(super) flash: u8,
}

impl Exit {
    pub(super) fn new(position: Vec3, web: &level::Web) -> Self {
        Self {
            speed: 0,
            extra_depth: 0,
            camera_z: web.camera_position(position).z,
            rear: web.rear(),
            web_end: 30.0 + f32::from(web.depth) + 70.0,
            warp_end: web.rear() + 1810 * LANE_UNIT,
            distance: 0,
            depth: ((0.12 - position.z) / WORLD_UNIT * LANE_UNIT as f32) as i32,
            ticks: 0,
            spin: 0,
            spacing: 0,
            wireframe: false,
            web_visible: true,
            flash: 0,
        }
    }

    fn advance(&mut self) -> bool {
        if self.flash > 0 {
            self.flash -= 1;
            return self.flash == 0;
        }
        self.ticks += 1;
        // zoom1 accelerates before running the claw; zoom2 runs it first.
        if self.web_visible {
            self.distance += self.speed;
            self.speed += ZOOM_ACCELERATION;
            self.depth += self.speed;
            if self.wireframe {
                self.spin = self.spin.wrapping_add(4);
                self.spacing += 0x4000;
            } else if self.depth >= self.rear {
                self.wireframe = true;
            }
            let source_camera =
                30.0 - self.camera_z / WORLD_UNIT + self.distance as f32 / LANE_UNIT as f32;
            if source_camera > self.web_end {
                self.web_visible = false;
            }
        } else {
            self.depth += self.speed + self.extra_depth;
            self.spin = self.spin.wrapping_add(4);
            // czoom2 and zoom2 each increase the trail spacing by a quarter unit.
            self.spacing += 0x8000;
            self.distance += self.speed;
            self.speed += ZOOM_ACCELERATION;
            self.extra_depth += 0x1000;
        }
        if self.depth >= self.warp_end {
            self.flash = FLASH_TICKS;
        }
        false
    }
}

#[derive(Component)]
pub(super) struct WarpFlash;

pub(super) fn setup(commands: &mut Commands) {
    commands.spawn((
        WarpFlash,
        BackgroundColor(Color::WHITE.with_alpha(0.0)),
        GlobalZIndex(100),
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            ..default()
        },
    ));
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn animate(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut player: Single<(&mut Glider, &mut Weapon, &mut Transform, &mut Visibility)>,
    mut cameras: Query<&mut Transform, (With<Camera3d>, Without<Glider>)>,
    actors: Query<
        (Entity, Option<&Bullet>),
        Or<(
            With<Bullet>,
            With<Flipper>,
            With<EnemyShot>,
            With<powerups::PowerUp>,
            With<explosion::Explosion>,
        )>,
    >,
    mut warp: Option<ResMut<stars::Warp>>,
    spikes: Query<&spikers::Spike>,
) {
    let Some(mut exit) = game.exit else {
        if let Some(warp) = &mut warp {
            **warp = stars::Warp::default();
        }
        return;
    };
    let (glider, weapon, transform, visibility) = &mut *player;
    let was_wireframe = exit.wireframe;
    if exit.advance() {
        game.exit = None;
        game.invulnerable = 0;
        game.load = Some(LoadReason::Advance);
        game.status = Status::Entering;
        **visibility = Visibility::Hidden;
        if let Some(warp) = &mut warp {
            **warp = stars::Warp::default();
        }
        return;
    }
    if exit.wireframe && !was_wireframe {
        for (entity, bullet) in &actors {
            if bullet.is_some() {
                commands.entity(entity).despawn();
            }
        }
    }
    transform.translation.z = 0.12 - exit.depth as f32 / LANE_UNIT as f32 * WORLD_UNIT;
    if exit.wireframe {
        glider.velocity = 0;
        **weapon = Weapon::default();
    }
    let target = level
        .web
        .camera_position(level.web.glider_transform(glider.progress).translation);
    for mut camera in &mut cameras {
        let delta = (target - camera.translation)
            .clamp(Vec3::splat(-CAMERA_STEP), Vec3::splat(CAMERA_STEP));
        camera.translation.x += delta.x;
        camera.translation.y += delta.y;
        camera.translation.z = exit.camera_z - exit.distance as f32 / LANE_UNIT as f32 * WORLD_UNIT;
    }
    if !exit.wireframe
        && exit.depth >= 0
        && spikes.iter().any(|spike| {
            spike.lane == level.web.selected_lane(glider.progress) as usize
                && exit.depth >= level.web.rear() - spike.length
        })
    {
        game.status = Status::Spiked;
        game.exit = None;
        game.death = Some(shot_death::Death::retreat(
            exit.camera_z - exit.distance as f32 / LANE_UNIT as f32 * WORLD_UNIT,
        ));
        glider.velocity = 0;
        **weapon = Weapon::default();
        if let Some(warp) = &mut warp {
            **warp = stars::Warp::default();
        }
        return;
    }
    if let Some(warp) = &mut warp {
        warp.distance = exit.distance;
        warp.spacing = i32::from(exit.ticks) * 0x4000;
        warp.copies = 3;
    }
    game.exit = Some(exit);
}

pub(super) fn draw(game: Res<Game>, mut flash: Single<&mut BackgroundColor, With<WarpFlash>>) {
    flash.0 = Color::WHITE.with_alpha(
        game.exit
            .map_or(0.0, |exit| f32::from(exit.flash) / f32::from(FLASH_TICKS)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_accelerates_then_switches_to_spinning_trails_at_the_rear() {
        let mut exit = Exit::new(
            glider_transform(START_PROGRESS).translation,
            &Level::default().web,
        );
        for _ in 0..64 {
            assert!(!exit.advance());
        }
        assert_eq!(exit.distance, 31 * LANE_UNIT + LANE_UNIT / 2);
        assert_eq!(exit.depth, 32 * LANE_UNIT + LANE_UNIT / 2);
        assert!(!exit.wireframe);
        for _ in 64..142 {
            exit.advance();
        }
        assert!(!exit.wireframe);
        exit.advance();
        assert_eq!(exit.depth, 160 * LANE_UNIT + 7 * LANE_UNIT / 8);
        assert!(exit.wireframe);
        assert!(exit.web_visible);
        exit.advance();
        assert_eq!(exit.spin, 4);
        assert_eq!(exit.spacing, LANE_UNIT / 4);
        for _ in 144..183 {
            exit.advance();
        }
        assert!(exit.web_visible);
        exit.advance();
        assert!(!exit.web_visible);
        assert_eq!(exit.spacing, 10 * LANE_UNIT + LANE_UNIT / 4);
        exit.advance();
        assert_eq!(exit.spacing, 10 * LANE_UNIT + 3 * LANE_UNIT / 4);
        for _ in 185..359 {
            exit.advance();
        }
        assert_eq!(exit.flash, 0);
        exit.advance();
        assert_eq!(exit.flash, FLASH_TICKS);
        assert!(exit.depth >= (2000 - 30) * LANE_UNIT);
        for _ in 0..FLASH_TICKS - 1 {
            assert!(!exit.advance());
        }
        assert!(exit.advance());
    }

    fn app() -> App {
        let mut app = App::new();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        entry::setup(
            &mut app.world_mut().commands(),
            &mut meshes,
            &mut materials,
            &Level::default().web,
        );
        setup(&mut app.world_mut().commands());
        app.world_mut().flush();
        let mut keys = ButtonInput::default();
        keys.press(KeyCode::KeyA);
        keys.press(KeyCode::Enter);
        app.insert_resource(Game {
            remaining: 0,
            score: 9000,
            lives: 2,
            ..default()
        })
        .insert_resource(keys)
        .init_resource::<Level>()
        .init_resource::<Levels>()
        .insert_resource(meshes)
        .insert_resource(materials)
        .init_resource::<powerups::Upgrades>()
        .init_resource::<ShotAssets>()
        .init_resource::<explosion::BurstAssets>()
        .init_resource::<stars::Warp>()
        .add_systems(
            FixedUpdate,
            (
                move_glider.run_if(controlling),
                fire_bullets.run_if(controlling),
                entry::advance,
                shot_death::animate,
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
        app
    }

    fn ticks(app: &mut App, count: usize) {
        for _ in 0..count {
            app.world_mut().run_schedule(FixedUpdate);
            app.world_mut().run_schedule(Update);
        }
    }

    #[test]
    fn exit_keeps_controls_until_the_rear_then_flashes_and_reenters_without_losing_rewards() {
        let mut app = app();
        app.world_mut().resource_mut::<powerups::Upgrades>().laser = true;
        app.world_mut()
            .resource_mut::<powerups::Upgrades>()
            .jump_enabled = true;
        ticks(&mut app, 1);
        assert!(app.world().resource::<Game>().status == Status::Exiting);
        ticks(&mut app, 64);
        assert_eq!(app.world().resource::<stars::Warp>().copies, 3);
        assert_eq!(
            app.world().resource::<stars::Warp>().spacing,
            16 * LANE_UNIT
        );
        assert!(app.world_mut().query::<&Bullet>().iter(app.world()).count() > 0);
        for camera in app
            .world_mut()
            .query_filtered::<&Transform, With<Camera3d>>()
            .iter(app.world())
        {
            assert!((camera.translation.z - (5.3 - 31.5 * WORLD_UNIT)).abs() < 1e-4);
        }
        ticks(&mut app, 79);
        let progress = app
            .world_mut()
            .query::<&Glider>()
            .single(app.world())
            .unwrap()
            .progress;
        assert_eq!(
            app.world_mut().query::<&Bullet>().iter(app.world()).count(),
            0
        );
        ticks(&mut app, 41);
        assert_eq!(
            app.world_mut()
                .query::<&Glider>()
                .single(app.world())
                .unwrap()
                .progress,
            progress
        );
        assert_eq!(
            *app.world_mut()
                .query_filtered::<&Visibility, With<entry::WebRoot>>()
                .single(app.world())
                .unwrap(),
            Visibility::Hidden
        );
        assert!(
            app.world_mut()
                .query_filtered::<&Visibility, With<entry::ClawTrail>>()
                .iter(app.world())
                .all(|visibility| *visibility == Visibility::Visible)
        );
        ticks(&mut app, 176);
        assert_eq!(
            app.world_mut()
                .query_filtered::<&BackgroundColor, With<WarpFlash>>()
                .single(app.world())
                .unwrap()
                .0
                .alpha(),
            1.0
        );
        ticks(&mut app, 16);
        let game = app.world().resource::<Game>();
        assert!(game.status == Status::Entering);
        assert_eq!(game.entry, Some(0));
        assert!(game.exit.is_none());
        assert_eq!((game.score, game.lives, game.remaining), (9000, 2, 25));
        assert!(app.world().resource::<powerups::Upgrades>().laser);
        assert!(app.world().resource::<powerups::Upgrades>().jump_enabled);
        assert_eq!(app.world().resource::<stars::Warp>().copies, 0);
        assert_eq!(
            app.world_mut()
                .query_filtered::<&BackgroundColor, With<WarpFlash>>()
                .single(app.world())
                .unwrap()
                .0
                .alpha(),
            0.0
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
    fn exits_advance_one_to_two_and_back_to_one_and_death_retries_two() {
        let mut app = app();
        let mut catalog =
            Levels::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("levels")).unwrap();
        catalog.0.truncate(2);
        app.insert_resource(catalog);
        ticks(&mut app, 377);
        assert_eq!(app.world().resource::<Level>().number, 2);
        assert_eq!(
            app.world_mut()
                .query::<&Glider>()
                .single(app.world())
                .unwrap()
                .progress,
            10 * LANE_UNIT
        );
        ticks(&mut app, 320);
        let progress = app
            .world_mut()
            .query::<&Glider>()
            .single(app.world())
            .unwrap()
            .progress;
        let lane = app.world().resource::<Level>().web.selected_lane(progress) as usize;
        app.world_mut().spawn((
            EnemyShot {
                lane,
                depth: LANE_UNIT,
                angle: 0,
            },
            Transform::default(),
        ));
        ticks(&mut app, 1);
        assert!(app.world().resource::<Game>().status == Status::ShotHit);
        ticks(&mut app, 163);
        assert_eq!(app.world().resource::<Level>().number, 2);
        assert_eq!(app.world().resource::<Game>().lives, 1);
        ticks(&mut app, 320);
        app.world_mut().resource_mut::<Game>().remaining = 0;
        ticks(&mut app, 377);
        assert_eq!(app.world().resource::<Level>().number, 1);
        assert_eq!(app.world().resource::<Game>().score, 9000);
        assert_eq!(app.world().resource::<Game>().lives, 1);
    }

    #[test]
    fn all_eight_levels_advance_through_the_exit_and_wrap_to_one() {
        let mut app = app();
        app.insert_resource(
            Levels::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("levels")).unwrap(),
        );
        for next in [2, 3, 4, 5, 6, 7, 8, 1] {
            app.world_mut().resource_mut::<Game>().remaining = 0;
            ticks(&mut app, 377);
            assert_eq!(app.world().resource::<Level>().number, next);
            assert_eq!(app.world().resource::<Game>().lives, 2);
            assert_eq!(app.world().resource::<Game>().score, 9000);
            assert!(app.world().resource::<Game>().status == Status::Entering);
            ticks(&mut app, 320);
            assert!(app.world().resource::<Game>().status == Status::Playing);
        }
    }

    #[test]
    fn restart_cancels_the_flight_and_restores_normal_stars() {
        let mut app = app();
        app.add_systems(Update, restart_game.before(entry::draw).before(draw));
        ticks(&mut app, 200);
        assert!(app.world().resource::<Game>().exit.is_some());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.world_mut().run_schedule(Update);
        assert!(app.world().resource::<Game>().exit.is_none());
        assert!(app.world().resource::<Game>().status == Status::Entering);
        assert_eq!(app.world().resource::<Game>().score, 0);
        assert_eq!(app.world().resource::<stars::Warp>().distance, 0);
    }
}
