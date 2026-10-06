use super::*;

#[derive(Resource)]
pub(crate) struct Upgrades {
    delay: u8,
    stage: u8,
    pub(crate) laser: bool,
    pub(crate) jump_enabled: bool,
    pub(crate) jump_height: i32,
    jump_velocity: i32,
    jump_queued: bool,
    pub(crate) droid: bool,
    warp_tokens: u8,
    frame: u32,
    notice: &'static str,
    notice_ticks: u16,
    random: u32,
}

impl Default for Upgrades {
    fn default() -> Self {
        Self {
            delay: 0,
            stage: 0,
            laser: false,
            jump_enabled: false,
            jump_height: 0,
            jump_velocity: 0,
            jump_queued: false,
            droid: false,
            warp_tokens: 0,
            frame: 0,
            notice: "",
            notice_ticks: 0,
            random: 0x2000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reward {
    Laser,
    Bonus,
    Jump,
    Droid,
    Zap,
    Skip,
}

impl Upgrades {
    pub(super) fn drop_due(&mut self, interval: u8) -> bool {
        if self.delay == 0 {
            self.delay = interval - 1;
            true
        } else {
            self.delay -= 1;
            false
        }
    }

    fn message(&mut self, message: &'static str) {
        self.notice = message;
        self.notice_ticks = 120;
    }

    fn collect(&mut self, score: &mut u32) -> Reward {
        let reward = match self.stage {
            0 => {
                self.laser = true;
                self.message("PARTICLE LASER!");
                Reward::Laser
            }
            2 => {
                self.jump_enabled = true;
                self.message("JUMP ENABLED!  J / SHIFT to jump");
                Reward::Jump
            }
            4 if !self.droid => {
                self.droid = true;
                self.message("A.I. DROID!");
                Reward::Droid
            }
            5 => {
                self.warp_tokens = (self.warp_tokens + 1).min(3);
                self.message(match self.warp_tokens {
                    1 => "SUPERZAP!  2 warp tokens to go",
                    2 => "SUPERZAP!  1 warp token to go",
                    _ => "SUPERZAP!  WARP READY!",
                });
                Reward::Zap
            }
            _ => {
                self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
                if self.random >> 24 <= 8 {
                    *score += 3000;
                    self.message("OUTTAH HERE!  +3000");
                    Reward::Skip
                } else {
                    *score += 2000;
                    self.message("2000 POINTS!");
                    Reward::Bonus
                }
            }
        };
        self.stage = (self.stage + 1).min(7);
        reward
    }

    pub(super) fn lose_life(&mut self) {
        let tokens = self.warp_tokens;
        *self = Self::default();
        self.warp_tokens = tokens;
    }

    pub(super) fn land(&mut self) {
        self.jump_height = 0;
        self.jump_velocity = 0;
        self.jump_queued = false;
    }

    pub(super) fn begin_level(&mut self) {
        self.delay = 0;
    }
}

#[derive(Component)]
#[require(LevelActor)]
pub(crate) struct PowerUp {
    lane: usize,
    depth: i32,
    rez: i16,
    radius: u16,
    pub(super) captured: bool,
    mesh: Option<Handle<Mesh>>,
}

impl PowerUp {
    pub(crate) fn preview_lane(&self) -> Option<usize> {
        (!self.captured && self.depth >= 0).then_some(self.lane)
    }

    fn tick(&mut self, lane: usize, grounded: bool) -> bool {
        if self.rez >= 0 {
            self.rez -= 4;
        } else if self.captured {
            self.radius += 5;
        } else if grounded && self.lane == lane && (0..=32 * LANE_UNIT).contains(&self.depth) {
            self.captured = true;
            self.radius = 11;
            return true;
        } else {
            self.depth -= 0x18000;
        }
        false
    }
}

#[derive(Component)]
#[require(LevelActor)]
pub(super) struct Droid {
    progress: i32,
    cooldown: u8,
}

#[derive(Resource, Default)]
pub(super) struct PowerAssets {
    material: Handle<StandardMaterial>,
    droid: Handle<Mesh>,
    droid_material: Handle<StandardMaterial>,
}

#[derive(Component)]
pub(super) struct PowerHud;

pub(super) fn setup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.insert_resource(PowerAssets {
        material: materials.add(StandardMaterial {
            unlit: true,
            cull_mode: None,
            ..default()
        }),
        droid: meshes.add(droid_mesh()),
        droid_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.85, 1.0),
            unlit: true,
            cull_mode: None,
            ..default()
        }),
    });
    commands.spawn((
        PowerHud,
        Text::default(),
        TextFont {
            font_size: 25.0,
            ..default()
        },
        TextColor(Color::srgb(0.4, 0.9, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: px(100),
            left: px(26),
            ..default()
        },
    ));
}

fn droid_mesh() -> Mesh {
    let corners: [Vec3; 8] = std::array::from_fn(|index| {
        Vec3::new(
            if index & 1 == 0 { -0.15 } else { 0.15 },
            if index & 2 == 0 { -0.15 } else { 0.15 },
            if index & 4 == 0 { -0.15 } else { 0.15 },
        )
    });
    let mut triangles = Vec::new();
    for index in 0..8 {
        for axis in [1, 2, 4] {
            if index & axis != 0 {
                continue;
            }
            let (a, b) = (corners[index], corners[index | axis]);
            let direction = (b - a).normalize();
            let u = direction
                .cross(if direction.z.abs() > 0.5 {
                    Vec3::Y
                } else {
                    Vec3::Z
                })
                .normalize()
                * 0.01;
            let v = direction.cross(u).normalize() * 0.01;
            let offsets = [-u - v, u - v, u + v, -u + v];
            for side in 0..4 {
                let next = (side + 1) % 4;
                let quad = [
                    a + offsets[side],
                    a + offsets[next],
                    b + offsets[next],
                    b + offsets[side],
                ]
                .map(|point| (point, Color::WHITE));
                triangles.extend([[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]]);
            }
        }
    }
    colored_mesh(triangles)
}

pub(super) fn spawn_drop(
    commands: &mut Commands,
    upgrades: &mut Upgrades,
    assets: &PowerAssets,
    lane: usize,
    depth: i32,
    web: &level::Web,
) {
    upgrades.message("COLLECT POWERUPS!");
    let mut transform = web.glider_transform(lane as i32 * LANE_UNIT);
    transform.translation.z = -(depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
    commands.spawn((
        PowerUp {
            lane,
            depth,
            rez: 300,
            radius: 10,
            captured: false,
            mesh: None,
        },
        Mesh3d::default(),
        MeshMaterial3d(assets.material.clone()),
        transform,
        RenderLayers::layer(1),
        bevy::camera::visibility::NoFrustumCulling,
    ));
}

pub(super) fn spawn_droid(commands: &mut Commands, assets: &PowerAssets, web: &level::Web) {
    let progress = (2 % web.lanes()) as i32 * LANE_UNIT;
    commands.spawn((
        Droid {
            progress,
            cooldown: 5,
        },
        Mesh3d(assets.droid.clone()),
        MeshMaterial3d(assets.droid_material.clone()),
        web.glider_transform(progress),
        RenderLayers::layer(1),
    ));
}

pub(super) fn queue_jump(keys: Res<ButtonInput<KeyCode>>, mut upgrades: ResMut<Upgrades>) {
    upgrades.jump_queued |= upgrades.jump_enabled
        && keys.any_just_pressed([KeyCode::KeyJ, KeyCode::ShiftLeft, KeyCode::ShiftRight]);
}

pub(super) fn update_jump(
    game: Res<Game>,
    keys: Res<ButtonInput<KeyCode>>,
    mut upgrades: ResMut<Upgrades>,
    mut player: Single<&mut Transform, With<Glider>>,
) {
    if game.status != Status::Playing {
        return;
    }
    let requested = std::mem::take(&mut upgrades.jump_queued)
        || keys.any_pressed([KeyCode::KeyJ, KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    if upgrades.jump_height == 0 && upgrades.jump_enabled && requested {
        upgrades.jump_velocity = 2 * LANE_UNIT;
    }
    if upgrades.jump_velocity != 0 {
        upgrades.jump_height += upgrades.jump_velocity;
        upgrades.jump_velocity -= 0xc11;
        if upgrades.jump_height <= 0 {
            upgrades.jump_height = 0;
            upgrades.jump_velocity = 0;
        }
    }
    player.translation.z = 0.12 + upgrades.jump_height as f32 / LANE_UNIT as f32 * WORLD_UNIT;
}

#[allow(clippy::too_many_arguments)]
pub(super) fn advance_powerups(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut upgrades: ResMut<Upgrades>,
    player: Single<&Glider>,
    assets: Res<PowerAssets>,
    bursts: Res<explosion::BurstAssets>,
    mut pickups: Query<(Entity, &mut PowerUp)>,
    enemies: Query<(Entity, &Flipper, &Transform)>,
    shots: Query<Entity, With<EnemyShot>>,
    tankers: Query<(Entity, &tankers::Tanker, &Transform)>,
    shells: Res<tankers::TankerAssets>,
    flippers: Res<FlipperAssets>,
    spikers: Query<(Entity, &spikers::Spiker, &Transform)>,
    fuses: Query<(Entity, &fuseballs::Fuseball, &Transform)>,
) {
    upgrades.frame = upgrades.frame.wrapping_add(1);
    upgrades.notice_ticks = upgrades.notice_ticks.saturating_sub(1);
    let lane = level.web.selected_lane(player.progress) as usize;
    for (entity, mut pickup) in &mut pickups {
        if (pickup.captured || game.status == Status::Playing)
            && pickup.tick(lane, upgrades.jump_height == 0)
        {
            match upgrades.collect(&mut game.score) {
                Reward::Droid => {
                    spawn_droid(&mut commands, &assets, &level.web);
                }
                reward @ (Reward::Zap | Reward::Skip) => {
                    for (entity, _, pose) in &fuses {
                        commands.entity(entity).despawn();
                        explosion::spawn_explosion(&mut commands, &bursts, pose.translation);
                        if reward == Reward::Zap {
                            game.score += 250;
                        }
                    }
                    for (entity, _, pose) in &spikers {
                        commands.entity(entity).despawn();
                        explosion::spawn_explosion(&mut commands, &bursts, pose.translation);
                        if reward == Reward::Zap {
                            game.score += 150;
                        }
                    }
                    for (entity, tanker, transform) in &tankers {
                        if reward == Reward::Zap {
                            tankers::open(
                                &mut commands,
                                &mut game,
                                &level,
                                &flippers,
                                &shells,
                                tanker,
                            );
                        }
                        commands.entity(entity).despawn();
                        explosion::spawn_explosion(&mut commands, &bursts, transform.translation);
                    }
                    for (enemy, _, transform) in &enemies {
                        commands.entity(enemy).despawn();
                        explosion::spawn_explosion(&mut commands, &bursts, transform.translation);
                        if reward == Reward::Zap {
                            game.score += 150;
                        }
                    }
                    for shot in &shots {
                        commands.entity(shot).despawn();
                    }
                    if reward == Reward::Skip {
                        game.remaining = 0;
                    }
                }
                _ => {}
            }
        }
        if pickup.radius >= 255 || pickup.depth < -80 * LANE_UNIT {
            if !pickup.captured {
                let mut centre = level
                    .web
                    .glider_transform(pickup.lane as i32 * LANE_UNIT)
                    .translation;
                centre.z = -(pickup.depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
                explosion::spawn_explosion(&mut commands, &bursts, centre);
            }
            commands.entity(entity).despawn();
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn move_droid(
    level: Res<Level>,
    mut commands: Commands,
    game: Res<Game>,
    upgrades: Res<Upgrades>,
    assets: Res<FlipperAssets>,
    shots: Res<ShotAssets>,
    enemies: Query<&Flipper>,
    tankers: Query<&tankers::Tanker>,
    spikers: Query<&spikers::Spiker>,
    fuses: Query<&fuseballs::Fuseball>,
    bullets: Query<Entity, With<ShotSlot>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    power_assets: Res<PowerAssets>,
    mut droids: Query<(&mut Droid, &mut Transform)>,
) {
    if game.status != Status::Playing {
        return;
    }
    if let Some(material) = materials.get_mut(&power_assets.droid_material) {
        material.base_color = cry::color(explosion::pulse_hue(upgrades.frame as u8), 255);
    }
    let target = enemies
        .iter()
        .filter(|flipper| !matches!(flipper.motion, Motion::Approach))
        .map(|flipper| (flipper.lane, flipper.depth))
        .chain(
            tankers
                .iter()
                .filter(|t| !t.approach)
                .map(|t| (t.lane, t.depth)),
        )
        .chain(spikers.iter().map(|s| (s.lane, s.depth)))
        .chain(fuses.iter().filter_map(|f| f.target()))
        .min_by_key(|(_, depth)| *depth)
        .map(|(lane, _)| lane);
    for (mut droid, mut transform) in &mut droids {
        if let Some(target) = target {
            let delta = level
                .web
                .lane_delta(droid.progress, target as i32 * LANE_UNIT);
            droid.progress = level
                .web
                .normalize(droid.progress + delta.clamp(-LANE_UNIT / 16, LANE_UNIT / 16));
        }
        let lane = droid.progress / LANE_UNIT;
        let fraction = (droid.progress % LANE_UNIT) as f32 / LANE_UNIT as f32;
        transform.translation = level
            .web
            .glider_transform(lane * LANE_UNIT)
            .translation
            .lerp(
                level
                    .web
                    .glider_transform((lane + 1) * LANE_UNIT)
                    .translation,
                fraction,
            )
            + Vec3::Z * (10.0 * WORLD_UNIT);
        let angle = upgrades.frame as f32 * TAU / 256.0;
        transform.rotation = Quat::from_euler(EulerRot::XYZ, angle, angle * 2.0, angle * 3.0);
        droid.cooldown = droid.cooldown.saturating_sub(1);
        if target.is_some() && droid.cooldown == 0 && bullets.iter().count() < MAX_BULLETS + 8 {
            let lane = level.web.selected_lane(droid.progress) as usize;
            let mut position = level
                .web
                .glider_transform(lane as i32 * LANE_UNIT)
                .translation;
            position.z = transform.translation.z;
            commands.spawn((
                Bullet { lane },
                RenderLayers::layer(1),
                Mesh3d(assets.shot.clone()),
                MeshMaterial3d(shots.material.clone()),
                Transform::from_translation(position),
            ));
            droid.cooldown = 6;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn add_ring(
    triangles: &mut Vec<[(Vec3, Color); 3]>,
    count: usize,
    radius: f32,
    depth: f32,
    phase: f32,
    color: Color,
    camera: &Transform,
    dot: f32,
) {
    if radius <= 0.0 {
        return;
    }
    let corner = |point: Vec3, x: f32, y: f32| {
        (
            point + camera.rotation * Vec3::new(x * dot, y * dot, 0.0),
            color,
        )
    };
    for index in 0..count {
        let angle = phase + index as f32 * TAU / count as f32;
        let point = camera.rotation * Vec3::new(angle.cos() * radius, angle.sin() * radius, depth);
        let quad = [
            corner(point, -1.0, -1.0),
            corner(point, 1.0, -1.0),
            corner(point, 1.0, 1.0),
            corner(point, -1.0, 1.0),
        ];
        triangles.extend([[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]]);
    }
}

#[allow(clippy::type_complexity)]
pub(super) fn draw_powerups(
    upgrades: Res<Upgrades>,
    mut meshes: ResMut<Assets<Mesh>>,
    cameras: Query<(&Transform, &Projection), (With<Camera3d>, Without<PowerUp>)>,
    mut pickups: Query<(&mut PowerUp, &mut Mesh3d, &mut Transform)>,
) {
    let Some((camera, Projection::Perspective(projection))) = cameras.iter().next() else {
        return;
    };
    let phase = upgrades.frame as f32 * TAU / 256.0;
    for (mut pickup, mut mesh, mut transform) in &mut pickups {
        transform.translation.z = -(pickup.depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
        transform.rotation = Quat::IDENTITY;
        let distance =
            -(camera.rotation.inverse() * (transform.translation - camera.translation)).z;
        let dot =
            distance.max(0.01) * (projection.fov * 0.5).tan() * projection.aspect_ratio / 384.0;
        let source_depth = (camera.translation.z - transform.translation.z) / WORLD_UNIT;
        let intensity = |depth: f32| (255.0 * (1.0 - depth / 650.0).clamp(0.0, 1.0)) as u8;
        let mut triangles = Vec::new();
        if pickup.rez >= 0 {
            add_ring(
                &mut triangles,
                20,
                pickup.rez as f32 * 0.06,
                0.0,
                phase,
                cry::color(
                    explosion::pulse_hue(upgrades.frame as u8),
                    intensity(source_depth),
                ),
                camera,
                dot,
            );
            for shift in 0..=3 {
                add_ring(
                    &mut triangles,
                    24,
                    ((300 - pickup.rez) >> shift) as f32 * 0.06,
                    0.0,
                    0.0,
                    cry::color(
                        explosion::pulse_hue(pickup.rez as u8),
                        (255 - ((300 - pickup.rez) >> shift)).clamp(0, 255) as u8,
                    ),
                    camera,
                    dot,
                );
            }
        } else {
            if !pickup.captured {
                let vertices = [
                    (7., 9.),
                    (12., 1.),
                    (12., 9.),
                    (13., 9.),
                    (9., 17.),
                    (9., 9.),
                ]
                .map(|(x, y)| camera.rotation * Vec3::new((x - 10.) * 0.06, (9. - y) * 0.06, 0.0));
                for face in [[0, 1, 2], [3, 4, 5]] {
                    triangles.push(std::array::from_fn(|index| {
                        let intensity = (u16::from([128u8, 192, 255][index])
                            * u16::from(intensity(source_depth))
                            / 255) as u8;
                        (vertices[face[index]], cry::color(0x8a, intensity))
                    }));
                }
            }
            for ring in 0..4 {
                let depth = ring as f32 * 8.0 * WORLD_UNIT;
                let hue = (source_depth as i32 - ring * 8) as u8;
                add_ring(
                    &mut triangles,
                    12,
                    pickup.radius as f32 * 0.06,
                    depth,
                    phase,
                    cry::color(
                        explosion::pulse_hue(hue),
                        intensity(source_depth - ring as f32 * 8.0),
                    ),
                    camera,
                    dot * ((distance - depth).max(0.01) / distance.max(0.01)),
                );
            }
        }
        let geometry = colored_mesh(triangles);
        if let Some(handle) = &pickup.mesh {
            if let Some(existing) = meshes.get_mut(handle) {
                *existing = geometry;
            }
        } else {
            pickup.mesh = Some(meshes.add(geometry));
        }
        mesh.0 = pickup.mesh.as_ref().unwrap().clone();
    }
}

pub(super) fn update_notice(upgrades: Res<Upgrades>, mut hud: Single<&mut Text, With<PowerHud>>) {
    let active = format!(
        "{}{}{}",
        if upgrades.laser { "LASER  " } else { "" },
        if upgrades.jump_enabled {
            "JUMP [J / SHIFT]  "
        } else {
            ""
        },
        if upgrades.droid { "DROID" } else { "" }
    );
    let notice = if upgrades.notice_ticks > 0 {
        upgrades.notice
    } else {
        ""
    };
    let text = format!("{active}\n{notice}");
    if hud.0 != text {
        hud.0 = text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pickup(depth: i32) -> PowerUp {
        PowerUp {
            lane: 3,
            depth,
            rez: 300,
            radius: 10,
            captured: false,
            mesh: None,
        }
    }

    #[test]
    fn configurable_drops_restart_the_counter_but_preserve_rewards_between_levels() {
        let mut upgrades = Upgrades::default();
        let drops = (1..=10)
            .filter(|_| upgrades.drop_due(3))
            .collect::<Vec<_>>();
        assert_eq!(drops, [1, 4, 7, 10]);
        upgrades.laser = true;
        upgrades.stage = 2;
        upgrades.begin_level();
        assert!(upgrades.drop_due(5));
        assert!(upgrades.laser);
        assert_eq!(upgrades.stage, 2);
    }

    #[test]
    fn first_kill_then_every_fifth_kill_drops_a_powerup() {
        let mut upgrades = Upgrades::default();
        let drops = (1..=25)
            .filter(|_| upgrades.drop_due(5))
            .collect::<Vec<_>>();
        assert_eq!(drops, [1, 6, 11, 16, 21]);
        assert_eq!(
            upgrades.stage, 0,
            "drops do not advance the reward sequence"
        );
    }

    #[test]
    fn a_drop_forms_for_76_ticks_then_moves_at_the_original_speed() {
        let mut pickup = pickup(100 * LANE_UNIT);
        for _ in 0..76 {
            assert!(!pickup.tick(3, true));
        }
        assert_eq!(pickup.depth, 100 * LANE_UNIT);
        assert_eq!(pickup.rez, -4);
        pickup.tick(3, true);
        assert_eq!(pickup.depth, 100 * LANE_UNIT - 0x18000);
    }

    #[test]
    fn pickup_requires_the_right_lane_range_and_a_grounded_claw() {
        let mut pickup = pickup(32 * LANE_UNIT);
        pickup.rez = -4;
        assert!(!pickup.tick(2, true));
        assert!(!pickup.tick(3, false));
        assert!(pickup.tick(3, true));
        assert_eq!(pickup.radius, 11);
        assert!(pickup.captured);
        let depth = pickup.depth;
        for _ in 0..49 {
            assert!(!pickup.tick(3, true));
        }
        assert_eq!(
            pickup.depth, depth,
            "the pickup burst stays at its collection point"
        );
        assert!(pickup.radius >= 255);
        let mut passed = super::tests::pickup(-LANE_UNIT);
        passed.rez = -4;
        assert!(!passed.tick(3, true), "a passed pickup cannot be collected");
    }

    #[test]
    fn rewards_follow_collection_order_and_death_clears_upgrades() {
        let mut upgrades = Upgrades::default();
        let mut score = 0;
        assert_eq!(upgrades.collect(&mut score), Reward::Laser);
        assert!(upgrades.laser);
        assert_eq!(upgrades.collect(&mut score), Reward::Bonus);
        assert_eq!(score, 2000);
        assert_eq!(upgrades.collect(&mut score), Reward::Jump);
        assert!(upgrades.jump_enabled);
        assert_eq!(upgrades.collect(&mut score), Reward::Bonus);
        assert_eq!(score, 4000);
        assert_eq!(upgrades.collect(&mut score), Reward::Droid);
        assert!(upgrades.droid);
        assert_eq!(upgrades.collect(&mut score), Reward::Zap);
        assert_eq!(upgrades.warp_tokens, 1);
        upgrades.lose_life();
        assert!(!upgrades.laser && !upgrades.jump_enabled && !upgrades.droid);
        assert_eq!(upgrades.stage, 0);
        assert_eq!(upgrades.warp_tokens, 1);
    }

    #[test]
    fn a_jump_tap_between_ticks_is_preserved_while_enter_is_held() {
        let mut app = App::new();
        app.insert_resource(Upgrades {
            jump_enabled: true,
            laser: true,
            ..default()
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<Game>()
        .init_resource::<Level>()
        .init_resource::<ShotAssets>()
        .add_systems(PreUpdate, queue_jump)
        .add_systems(
            FixedUpdate,
            (move_glider, update_jump, fire_bullets).chain(),
        );
        app.world_mut()
            .spawn((Glider::default(), Weapon::default(), Transform::default()));
        for key in [KeyCode::KeyJ, KeyCode::ShiftLeft] {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::Enter);
            keys.press(key);
            app.world_mut().run_schedule(PreUpdate);
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .release(key);
            app.world_mut().run_schedule(FixedUpdate);
            assert!(app.world().resource::<Upgrades>().jump_height > 0);
            assert!(
                app.world_mut()
                    .query::<&Bullet>()
                    .iter(app.world())
                    .next()
                    .is_some()
            );
            let mut upgrades = app.world_mut().resource_mut::<Upgrades>();
            upgrades.jump_height = 0;
            upgrades.jump_velocity = 0;
        }
    }

    #[test]
    fn jump_lands_and_resets_its_velocity() {
        let mut app = App::new();
        let upgrades = Upgrades {
            jump_enabled: true,
            ..default()
        };
        let mut keys = ButtonInput::default();
        keys.press(KeyCode::KeyJ);
        app.insert_resource(upgrades)
            .insert_resource(keys)
            .init_resource::<Game>()
            .init_resource::<Level>()
            .add_systems(FixedUpdate, update_jump);
        app.world_mut()
            .spawn((Glider::default(), Transform::default()));
        app.world_mut().run_schedule(FixedUpdate);
        assert!(app.world().resource::<Upgrades>().jump_height > 0);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyJ);
        for _ in 0..100 {
            app.world_mut().run_schedule(FixedUpdate);
        }
        let upgrades = app.world().resource::<Upgrades>();
        assert_eq!(upgrades.jump_height, 0);
        assert_eq!(upgrades.jump_velocity, 0);
    }

    #[test]
    fn collection_grants_one_reward_and_the_burst_expires() {
        let mut app = App::new();
        app.init_resource::<Game>()
            .init_resource::<Level>()
            .init_resource::<tankers::TankerAssets>()
            .init_resource::<FlipperAssets>()
            .init_resource::<Upgrades>()
            .init_resource::<PowerAssets>()
            .init_resource::<explosion::BurstAssets>()
            .add_systems(FixedUpdate, advance_powerups);
        app.world_mut().spawn(Glider {
            progress: 3 * LANE_UNIT,
            velocity: 0,
        });
        let mut pickup = pickup(30 * LANE_UNIT);
        pickup.rez = -4;
        let entity = app.world_mut().spawn(pickup).id();
        app.world_mut().run_schedule(FixedUpdate);
        assert!(app.world().resource::<Upgrades>().laser);
        assert_eq!(app.world().resource::<Upgrades>().stage, 1);
        for _ in 0..48 {
            app.world_mut().run_schedule(FixedUpdate);
        }
        assert!(app.world().get::<PowerUp>(entity).is_some());
        app.world_mut().run_schedule(FixedUpdate);
        assert!(app.world().get::<PowerUp>(entity).is_none());
        assert_eq!(app.world().resource::<Upgrades>().stage, 1);
    }

    #[test]
    fn droid_seeks_the_closest_enemy_and_fires_every_six_ticks() {
        let mut app = App::new();
        app.init_resource::<Game>()
            .init_resource::<Level>()
            .init_resource::<Upgrades>()
            .init_resource::<FlipperAssets>()
            .init_resource::<ShotAssets>()
            .init_resource::<PowerAssets>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(FixedUpdate, move_droid);
        app.world_mut().spawn(Flipper {
            lane: 3,
            depth: 80 * LANE_UNIT,
            motion: Motion::Climb,
            pause: 0,
        });
        app.world_mut().spawn(Flipper {
            lane: 10,
            depth: 140 * LANE_UNIT,
            motion: Motion::Climb,
            pause: 0,
        });
        let droid = app
            .world_mut()
            .spawn((
                Droid {
                    progress: 2 * LANE_UNIT,
                    cooldown: 0,
                },
                Transform::default(),
            ))
            .id();
        for _ in 0..16 {
            app.world_mut().run_schedule(FixedUpdate);
        }
        assert_eq!(
            app.world().get::<Droid>(droid).unwrap().progress,
            3 * LANE_UNIT
        );
        assert_eq!(
            app.world_mut().query::<&Bullet>().iter(app.world()).count(),
            3
        );
    }
}
