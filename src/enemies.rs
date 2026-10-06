use std::{collections::HashSet, f32::consts::TAU};

use super::*;

use crate::explosion;

#[path = "powerups.rs"]
pub(crate) mod powerups;

#[path = "entry.rs"]
pub(crate) mod entry;

#[path = "exit.rs"]
mod exit;

#[path = "shot_death.rs"]
mod shot_death;

#[path = "tankers.rs"]
pub(crate) mod tankers;

#[path = "spikers.rs"]
pub(crate) mod spikers;

#[path = "beasts.rs"]
mod beasts;

#[derive(Component, Default)]
struct Enemy;

#[cfg(test)]
const REAR: i32 = 160 * LANE_UNIT;
const RETREAT_END: f32 = (110.0 - 80.0 + 400.0) * WORLD_UNIT;
#[cfg(test)]
const MAX_FLIPPERS: usize = 11;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Entering,
    Playing,
    Captured,
    ShotHit,
    Spiked,
    Respawning,
    Exiting,
    GameOver,
}

#[derive(Clone, Copy)]
struct Capture {
    flipper: Entity,
    depth: u16,
    camera_z: f32,
}

#[derive(Resource)]
struct Game {
    score: u32,
    lives: u8,
    status: Status,
    remaining: u16,
    spawn_timer: u16,
    invulnerable: u16,
    random: u32,
    fire_timer: u8,
    capture: Option<Capture>,
    entry: Option<u16>,
    exit: Option<exit::Exit>,
    death: Option<shot_death::Death>,
    level_index: usize,
    load: Option<LoadReason>,
    tanker_timer: u16,
    spiker_timer: u16,
    beast_timer: u16,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            score: 0,
            lives: 3,
            status: Status::Playing,
            // _nw1's counter runs from 24 through zero before becoming negative.
            remaining: 25,
            spawn_timer: 60,
            invulnerable: 0,
            random: 0x2000,
            fire_timer: 112,
            capture: None,
            entry: None,
            exit: None,
            death: None,
            level_index: 0,
            load: None,
            tanker_timer: 0,
            spiker_timer: 0,
            beast_timer: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Motion {
    Approach,
    Climb,
    Flip {
        from: usize,
        direction: i32,
        angle: u16,
    },
    Pause(u8),
}

#[derive(Component, Debug)]
#[require(LevelActor, Enemy)]
pub(crate) struct Flipper {
    lane: usize,
    depth: i32,
    motion: Motion,
    pause: u8,
}

#[derive(Component)]
#[require(LevelActor)]
struct EnemyShot {
    lane: usize,
    depth: i32,
    angle: u8,
}

impl Flipper {
    pub(crate) fn preview_target(&self, web: &level::Web) -> Option<(usize, i32)> {
        self.hit_lane(web).map(|lane| (lane, self.depth))
    }

    fn hit_lane(&self, web: &level::Web) -> Option<usize> {
        match self.motion {
            Motion::Approach => None,
            Motion::Flip {
                from,
                direction,
                angle,
            } if web
                .flip_angle(from, self.lane, direction)
                .saturating_sub(angle)
                > 52 =>
            {
                None
            }
            _ => Some(self.lane),
        }
    }

    fn danger_lane(&self, web: &level::Web) -> Option<usize> {
        if self.depth > 2 * LANE_UNIT {
            return None;
        }
        match self.motion {
            Motion::Pause(_) => Some(self.lane),
            Motion::Flip {
                from,
                direction,
                angle,
            } if web
                .flip_angle(from, self.lane, direction)
                .saturating_sub(angle)
                > 52 =>
            {
                Some(from)
            }
            Motion::Flip { .. } => Some(self.lane),
            _ => None,
        }
    }
}

#[derive(Resource, Default)]
struct FlipperAssets {
    body: Handle<Mesh>,
    point: Handle<Mesh>,
    shaded: Handle<StandardMaterial>,
    point_shades: Vec<Handle<StandardMaterial>>,
    shot: Handle<Mesh>,
}

#[derive(Component)]
struct GameHud;

pub(super) fn add_systems(app: &mut App) {
    let initial = app
        .world()
        .get_resource::<Level>()
        .cloned()
        .unwrap_or_default();
    let index = app
        .world()
        .get_resource::<StartingLevel>()
        .map_or(0, |start| start.0);
    app.init_resource::<Level>()
        .init_resource::<Levels>()
        .insert_resource(Game {
            status: Status::Entering,
            entry: Some(0),
            remaining: initial.flippers.count,
            spawn_timer: initial.flippers.first_spawn_ticks - 1,
            level_index: index,
            tanker_timer: initial
                .tankers
                .as_ref()
                .map_or(0, |g| g.first_spawn_ticks - 1),
            spiker_timer: initial
                .spikers
                .as_ref()
                .map_or(0, |g| g.first_spawn_ticks - 1),
            beast_timer: initial
                .beasts
                .as_ref()
                .map_or(0, |g| g.first_spawn_ticks - 1),
            ..default()
        })
        .init_resource::<powerups::Upgrades>()
        .add_systems(
            PreUpdate,
            powerups::queue_jump.after(bevy::input::InputSystems),
        )
        .add_systems(
            FixedUpdate,
            preview_transition.after(preview_input).before(move_glider),
        )
        .add_systems(
            FixedUpdate,
            (
                (
                    move_glider.run_if(controlling),
                    powerups::update_jump,
                    animate_glider,
                    update_lane,
                    follow_glider.run_if(|game: Res<Game>| {
                        !matches!(
                            game.status,
                            Status::Entering
                                | Status::Captured
                                | Status::ShotHit
                                | Status::Spiked
                                | Status::Exiting
                        )
                    }),
                    move_bullets,
                    fire_bullets.run_if(controlling),
                    spawn_enemies,
                    tankers::spawn,
                    spikers::spawn,
                    beasts::spawn,
                    move_enemies,
                    move_enemy_shots,
                    resolve_hits,
                    tankers::move_tankers,
                    spikers::advance,
                    fire_enemy_shots,
                    powerups::advance_powerups,
                    powerups::move_droid,
                )
                    .chain(),
                (
                    explosion::age_explosions,
                    entry::advance,
                    animate_capture,
                    shot_death::animate,
                    exit::animate,
                    check_game,
                    load_level,
                )
                    .chain(),
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                fit_camera,
                quit_on_escape,
                restart_game,
                load_level,
                update_hud,
            )
                .chain(),
        )
        .add_systems(Update, sync_web.after(load_level))
        .add_systems(Update, update_enemy_colors.after(load_level))
        .add_systems(Update, draw_approaching_flippers.after(fit_camera))
        .add_systems(Update, tankers::draw_points.after(fit_camera))
        .add_systems(Update, draw_particle_shots.after(fit_camera))
        .add_systems(
            Update,
            (
                powerups::draw_powerups.after(fit_camera),
                entry::draw.after(restart_game),
                exit::draw.after(restart_game),
                shot_death::draw.after(restart_game),
                spikers::draw.after(restart_game),
                powerups::update_notice,
            ),
        );
}

fn playing(game: Res<Game>) -> bool {
    game.status == Status::Playing
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn preview_transition(
    level: Res<Level>,
    mut commands: Commands,
    capture: Option<Res<MotionCapture>>,
    mut game: ResMut<Game>,
    actors: Query<Entity, With<LevelActor>>,
    player: Single<&Glider>,
    assets: Res<FlipperAssets>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
) {
    let Some(capture) = capture else {
        return;
    };
    if capture.tick == 360 && capture.preview != CapturePreview::Motion {
        for entity in &actors {
            commands.entity(entity).despawn();
        }
        if capture.preview == CapturePreview::Exit {
            game.remaining = 0;
        } else {
            for key in [KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Space] {
                keys.release(key);
            }
            let shot = EnemyShot {
                lane: level.web.selected_lane(player.progress) as usize,
                depth: 2 * LANE_UNIT + LANE_UNIT / 2,
                angle: 0,
            };
            let mut transform = level.web.glider_transform(shot.lane as i32 * LANE_UNIT);
            transform.translation.z = -(shot.depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
            commands.spawn((
                shot,
                transform,
                RenderLayers::layer(1),
                Mesh3d(assets.shot.clone()),
                MeshMaterial3d(assets.shaded.clone()),
            ));
        }
    }
}

fn controlling(game: Res<Game>) -> bool {
    game.exit
        .is_some_and(|exit| !exit.wireframe && exit.flash == 0)
        || playing(game)
}

#[cfg(test)]
const BRIGHTEST_RED: u8 = 240;

fn flipper_mesh(color: [u8; 3]) -> Mesh {
    // A Flipper's four tips sit on the corners of its bounding box, so that box
    // decides the whole look. `fverts` uses 16 by 18, which is nearly square, so
    // the X closes towards a plus sign. Sampling both references gives a much
    // shallower cross: the arms meet at roughly 58 degrees, which needs a box
    // about twice as wide as it is tall. So the tips move to (0,0) and (32,16).
    //
    // The inner vertices keep the source's proportion. There they sit halfway
    // between the centre and a tip, which makes each arm about a quarter as
    // wide as it is long. The reference measures the same ratio, so the inner
    // points move out to (8,8) and (24,8) to match.
    //
    // This is a deliberate departure from fverts, made to match how the
    // original actually renders.
    let vertices = [
        (16., 8.),
        (8., 8.),
        (24., 8.),
        (0., 0.),
        (32., 0.),
        (0., 16.),
        (32., 16.),
    ]
    .map(|(x, y)| Vec3::new((x - 16.) * 0.06, (8. - y) * 0.06, 0.0));
    // The original intensities for each face's tip, inner, and centre vertices.
    let faces = [
        ([3, 1, 0], [128u16, 192, 255]),
        ([4, 2, 0], [64, 128, 192]),
        ([5, 1, 0], [128, 192, 255]),
        ([6, 2, 0], [64, 128, 192]),
    ];
    // Measured against the original renderer, a Flipper reads as four flat
    // pure-red facets: no green channel, and no gradient along an arm.
    colored_mesh(faces.map(|(face, intensity)| {
        let red = intensity.iter().copied().max().unwrap();
        let [r, g, b] = color.map(|channel| (red * u16::from(channel) / 255) as u8);
        let color = Color::srgb_u8(r, g, b);
        face.map(|vertex| (vertices[vertex], color))
    }))
}

fn enemy_shot_mesh() -> Mesh {
    let vertices = [(1., 9.), (4., 9.), (5., 8.), (5., 10.)]
        .map(|(x, y)| Vec3::new((x - 9.) * 0.075, (9. - y) * 0.075, 0.0));
    let faces = [
        ([135, 104, 255], [(2, 192), (1, 255), (0, 64)]),
        ([224, 255, 71], [(3, 192), (1, 255), (0, 64)]),
    ];
    let mut triangles = Vec::new();
    for turn in 0..3 {
        let rotation = Quat::from_rotation_z(-(turn * 85) as f32 * TAU / 256.0);
        for (rgb, face) in faces {
            triangles.push(face.map(|(index, intensity)| {
                (
                    rotation * vertices[index],
                    Color::srgb_u8(
                        ((rgb[0] * intensity) >> 8) as u8,
                        ((rgb[1] * intensity) >> 8) as u8,
                        ((rgb[2] * intensity) >> 8) as u8,
                    ),
                )
            }));
        }
    }
    colored_mesh(triangles)
}

pub(super) fn setup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    level: &Level,
) {
    explosion::setup(commands, meshes, materials);
    powerups::setup(commands, meshes, materials);
    exit::setup(commands);
    shot_death::setup(commands, meshes, materials);
    tankers::setup(commands, meshes, materials);
    spikers::setup(commands, meshes, materials);
    beasts::setup(commands, meshes);
    commands.insert_resource(FlipperAssets {
        body: meshes.add(flipper_mesh(level.appearance.flipper)),
        point: meshes.add(Rectangle::new(1.0, 1.0)),
        shot: meshes.add(enemy_shot_mesh()),
        shaded: materials.add(StandardMaterial {
            unlit: true,
            cull_mode: None,
            ..default()
        }),
        point_shades: (0..=255u16)
            .map(|intensity| {
                materials.add(StandardMaterial {
                    base_color: flipper_dot_color(level.appearance.flipper, intensity),
                    unlit: true,
                    cull_mode: None,
                    ..default()
                })
            })
            .collect(),
    });
    commands.spawn((
        GameHud,
        Text::default(),
        TextFont {
            font_size: 22.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.85, 0.25)),
        Node {
            position_type: PositionType::Absolute,
            left: px(26),
            top: px(22),
            ..default()
        },
    ));
}

fn flipper_dot_color(color: [u8; 3], intensity: u16) -> Color {
    let [r, g, b] = color.map(|channel| {
        (u32::from(channel) * 254 * u32::from(intensity) / (240 * 256)).min(255) as u8
    });
    Color::srgb_u8(r, g, b)
}

fn update_enemy_colors(
    level: Res<Level>,
    assets: Res<FlipperAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !level.is_changed() {
        return;
    }
    if let Some(body) = meshes.get_mut(&assets.body) {
        *body = flipper_mesh(level.appearance.flipper);
    }
    for (intensity, handle) in assets.point_shades.iter().enumerate() {
        if let Some(material) = materials.get_mut(handle) {
            material.base_color = flipper_dot_color(level.appearance.flipper, intensity as u16);
        }
    }
}

fn flipper_transform(flipper: &Flipper, web: &level::Web) -> Transform {
    let z = -(flipper.depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
    if let Motion::Flip {
        from,
        direction,
        angle,
    } = flipper.motion
    {
        let pivot = web.vertex(
            (from as i32 + i32::from(direction > 0)) as f32,
            FRONT_RADIUS,
            z,
        );
        let rotation = web.orientation(from)
            * Quat::from_rotation_z(direction as f32 * angle as f32 * TAU / 256.0);
        return Transform::from_translation(
            pivot + rotation * Vec3::X * (-direction as f32 * 9.0 * 0.075),
        )
        .with_rotation(rotation);
    }
    let mut transform = web.glider_transform(flipper.lane as i32 * LANE_UNIT);
    transform.translation.z = z;
    transform.rotation = web.orientation(flipper.lane);
    transform
}

fn spawn_enemies(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    assets: Res<FlipperAssets>,
    enemies: Query<&Flipper>,
    tankers: Query<&tankers::Tanker>,
    spikers: Query<&spikers::Spiker>,
) {
    if game.status != Status::Playing || game.remaining == 0 {
        return;
    }
    if game.spawn_timer > 0 {
        game.spawn_timer -= 1;
        return;
    }
    if enemies.iter().count() + tankers.iter().count() * 3 + spikers.iter().count()
        >= level.flippers.max_active
    {
        return;
    }
    game.random = game.random.wrapping_mul(1664525).wrapping_add(1013904223);
    let flipper = Flipper {
        lane: (game.random >> 16) as usize % level.web.lanes(),
        depth: level.web.rear() + 300 * LANE_UNIT,
        motion: Motion::Approach,
        pause: level.flippers.pause_ticks,
    };
    commands.spawn((
        flipper_transform(&flipper, &level.web),
        flipper,
        RenderLayers::layer(1),
        Mesh3d(assets.point.clone()),
        MeshMaterial3d(assets.point_shades.first().cloned().unwrap_or_default()),
    ));
    game.remaining -= 1;
    game.spawn_timer = level.flippers.spawn_interval_ticks - 1;
}

fn start_flip(flipper: &mut Flipper, player_lane: usize, web: &level::Web) {
    if flipper.lane == player_lane {
        flipper.motion = Motion::Pause(flipper.pause);
        return;
    }
    let lanes = web.lanes();
    let clockwise = (player_lane + lanes - flipper.lane) % lanes;
    let direction =
        if (!web.closed && player_lane > flipper.lane) || (web.closed && clockwise < lanes / 2) {
            1
        } else {
            -1
        };
    let from = flipper.lane;
    flipper.lane = web.neighbor(from, direction).unwrap();
    flipper.motion = Motion::Flip {
        from,
        direction,
        angle: 0,
    };
}

#[allow(clippy::type_complexity)]
fn move_enemies(
    level: Res<Level>,
    game: Res<Game>,
    player: Single<&Glider>,
    assets: Res<FlipperAssets>,
    beast_assets: Res<beasts::BeastAssets>,
    mut enemies: Query<(
        &mut Flipper,
        &mut Transform,
        &mut Mesh3d,
        &mut MeshMaterial3d<StandardMaterial>,
        Option<&mut tankers::Released>,
        Option<&beasts::Beast>,
    )>,
) {
    if game.status != Status::Playing {
        return;
    }
    let player_lane = level.web.selected_lane(player.progress) as usize;
    for (mut flipper, mut transform, mut mesh, mut material, mut released, beast) in &mut enemies {
        let speed = if beast.is_some() {
            level
                .beasts
                .as_ref()
                .map_or(level.flippers.climb_speed, |g| g.climb_speed)
        } else {
            level.flippers.climb_speed
        };
        match flipper.motion {
            Motion::Approach => {
                flipper.depth -= 2 * LANE_UNIT;
                if flipper.depth <= level.web.rear() {
                    flipper.depth = level.web.rear() - speed;
                    flipper.motion = Motion::Climb;
                    mesh.0 = beast.map_or_else(
                        || assets.body.clone(),
                        |beast| beast_assets.frames[beast.stage].clone(),
                    );
                    material.0 = assets.shaded.clone();
                }
            }
            Motion::Climb => {
                flipper.depth = (flipper.depth - speed).max(0);
                if flipper.depth == 0 {
                    start_flip(&mut flipper, player_lane, &level.web);
                }
            }
            Motion::Flip {
                from,
                direction,
                angle,
            } => {
                let angle = angle
                    + level.flippers.flip_speed
                        * if released.as_ref().is_some_and(|r| r.fast) {
                            2
                        } else {
                            1
                        };
                flipper.motion = if angle >= level.web.flip_angle(from, flipper.lane, direction) {
                    Motion::Pause(if released.as_ref().is_some_and(|r| r.burst) {
                        0
                    } else {
                        flipper.pause
                    })
                } else {
                    Motion::Flip {
                        from,
                        direction,
                        angle,
                    }
                };
            }
            Motion::Pause(0) => {
                if let Some(released) = &mut released
                    && released.burst
                {
                    if released.remaining == 0 {
                        released.burst = false;
                        flipper.motion = Motion::Climb;
                    } else {
                        released.remaining -= 1;
                        let from = flipper.lane;
                        let direction = if level.web.neighbor(from, released.direction).is_none() {
                            -released.direction
                        } else {
                            released.direction
                        };
                        flipper.lane = level.web.neighbor(from, direction).unwrap();
                        flipper.motion = Motion::Flip {
                            from,
                            direction,
                            angle: 0,
                        };
                    }
                } else {
                    start_flip(&mut flipper, player_lane, &level.web);
                }
            }
            Motion::Pause(ticks) => flipper.motion = Motion::Pause(ticks - 1),
        }
        *transform = flipper_transform(&flipper, &level.web);
        if beast.is_some()
            && let Motion::Flip {
                from,
                direction,
                angle,
            } = flipper.motion
        {
            transform.rotation = level.web.orientation(from)
                * Quat::from_rotation_z(direction as f32 * angle as f32 * TAU / 128.);
        }
    }
}

fn approach_dot_style(
    position: Vec3,
    camera: &Transform,
    projection: &PerspectiveProjection,
) -> (f32, usize) {
    let distance = -(camera.rotation.inverse() * (position - camera.translation)).z;
    let width =
        4.0 * distance.max(0.01) * (projection.fov * 0.5).tan() * projection.aspect_ratio / 384.0;
    // snglpix doubles z_max=1300, in quarter-units, for its depth cue.
    let source_depth = (camera.translation.z - position.z).max(0.0) / WORLD_UNIT;
    let intensity = (255.0 * (1.0 - source_depth / 650.0).clamp(0.0, 1.0)) as usize;
    (width, intensity)
}

#[allow(clippy::type_complexity)]
fn draw_approaching_flippers(
    assets: Res<FlipperAssets>,
    cameras: Query<(&Transform, &Projection), (With<Camera3d>, Without<Flipper>)>,
    mut enemies: Query<(
        &Flipper,
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let Some((camera, Projection::Perspective(projection))) = cameras.iter().next() else {
        return;
    };
    for (flipper, mut transform, mut material) in &mut enemies {
        if matches!(flipper.motion, Motion::Approach) {
            let (width, intensity) = approach_dot_style(transform.translation, camera, projection);
            transform.rotation = camera.rotation;
            transform.scale = Vec3::splat(width);
            if let Some(shade) = assets.point_shades.get(intensity) {
                material.0 = shade.clone();
            }
        }
    }
}

fn fire_enemy_shots(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    assets: Res<FlipperAssets>,
    enemies: Query<&Flipper>,
    shots: Query<(&EnemyShot, Option<&beasts::Reflected>)>,
    spikers: Query<&spikers::Spiker>,
) {
    if game.status != Status::Playing {
        return;
    }
    let sources = enemies
        .iter()
        .filter(|f| matches!(f.motion, Motion::Climb))
        .map(|f| (f.lane, f.depth))
        .chain(
            spikers
                .iter()
                .filter(|s| s.firing())
                .map(|s| (s.lane, s.depth)),
        );
    for (lane, depth) in sources {
        if game.fire_timer > 0 {
            game.fire_timer -= 1;
            continue;
        }
        game.fire_timer = 112;
        if depth <= 60 * LANE_UNIT
            || shots
                .iter()
                .filter(|(_, reflection)| reflection.is_none())
                .count()
                >= level.enemy_shots
        {
            continue;
        }
        let shot = EnemyShot {
            lane,
            depth,
            angle: 0,
        };
        commands.spawn((
            Transform::from_translation({
                let mut p = level
                    .web
                    .glider_transform(lane as i32 * LANE_UNIT)
                    .translation;
                p.z = -(depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
                p
            }),
            shot,
            RenderLayers::layer(1),
            Mesh3d(assets.shot.clone()),
            MeshMaterial3d(assets.shaded.clone()),
        ));
        break;
    }
}

fn move_enemy_shots(
    mut commands: Commands,
    game: Res<Game>,
    mut shots: Query<(
        Entity,
        &mut EnemyShot,
        &mut Transform,
        Option<&beasts::Reflected>,
    )>,
) {
    if game.status != Status::Playing {
        return;
    }
    for (entity, mut shot, mut transform, reflection) in &mut shots {
        shot.depth -= reflection.map_or(0x18000, |reflection| reflection.speed);
        shot.angle = shot.angle.wrapping_add(4);
        transform.translation.z = -(shot.depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
        transform.rotation = Quat::from_rotation_z(-(shot.angle as f32) * TAU / 256.0);
        if shot.depth < 0 {
            commands.entity(entity).despawn();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_hits(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    bursts: Res<explosion::BurstAssets>,
    mut upgrades: ResMut<powerups::Upgrades>,
    power_assets: Res<powerups::PowerAssets>,
    bullets: Query<(Entity, &Bullet, &Transform, Option<&ParticleShot>)>,
    enemies: Query<(Entity, &Flipper, &Transform)>,
    enemy_shots: Query<(Entity, &EnemyShot, Option<&beasts::Reflected>)>,
    tankers: Query<(Entity, &tankers::Tanker, &Transform)>,
    shells: Res<tankers::TankerAssets>,
    flipper_assets: Res<FlipperAssets>,
    spikers: Query<(Entity, &spikers::Spiker, &Transform)>,
    mut spikes: Query<(Entity, &mut spikers::Spike)>,
    mut beast_layers: Query<(&mut beasts::Beast, &Flipper, &mut Mesh3d)>,
    beast_assets: Res<beasts::BeastAssets>,
) {
    if !matches!(game.status, Status::Playing | Status::Exiting) {
        return;
    }
    let mut killed = HashSet::new();
    for (shot, bullet, transform, particle) in &bullets {
        let intersects = |lane, depth: i32| {
            let z = -(depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
            lane == bullet.lane && (z - transform.translation.z).abs() < WORLD_UNIT * 6.0
        };
        let hits = enemies
            .iter()
            .filter(|(entity, flipper, _)| {
                !killed.contains(entity)
                    && flipper.hit_lane(&level.web) == Some(bullet.lane)
                    && intersects(flipper.lane, flipper.depth)
            })
            .map(|(entity, flipper, transform)| {
                (
                    entity,
                    flipper.depth,
                    150,
                    Some((flipper.lane, transform.translation)),
                )
            })
            .chain(
                tankers
                    .iter()
                    .filter(|(entity, tanker, _)| {
                        !killed.contains(entity)
                            && !tanker.approach
                            && intersects(tanker.lane, tanker.depth)
                    })
                    .map(|(entity, tanker, _)| (entity, tanker.depth, 0, None)),
            )
            .chain(
                spikers
                    .iter()
                    .filter(|(entity, s, _)| {
                        !killed.contains(entity) && intersects(s.lane, s.depth)
                    })
                    .map(|(entity, s, pose)| {
                        (entity, s.depth, 150, Some((s.lane, pose.translation)))
                    }),
            )
            .chain(
                spikes
                    .iter()
                    .filter(|(entity, s)| {
                        !killed.contains(entity) && intersects(s.lane, level.web.rear() - s.length)
                    })
                    .map(|(entity, s)| (entity, level.web.rear() - s.length, 0, None)),
            )
            .chain(
                enemy_shots
                    .iter()
                    .filter(|(entity, shot, _)| {
                        !killed.contains(entity) && intersects(shot.lane, shot.depth)
                    })
                    .map(|(entity, shot, _)| (entity, shot.depth, 0, None)),
            )
            .collect::<Vec<_>>();
        let nearest = hits
            .iter()
            .min_by_key(|(_, depth, _, _)| *depth)
            .map(|hit| hit.0);
        for (entity, depth, points, centre) in hits {
            if particle.is_none() && Some(entity) != nearest {
                continue;
            }
            if particle.is_none() {
                commands.entity(shot).despawn();
            }
            if enemy_shots
                .get(entity)
                .is_ok_and(|(_, _, reflection)| reflection.is_some())
            {
                continue;
            }
            if let Ok((mut beast, flipper, mut mesh)) = beast_layers.get_mut(entity)
                && beast.stage > 0
                && matches!(flipper.motion, Motion::Climb)
            {
                let part = beast_assets.parts[beast.stage - 1].clone();
                beast.stage -= 1;
                mesh.0 = beast_assets.frames[beast.stage].clone();
                commands.entity(shot).despawn();
                commands.spawn((
                    EnemyShot {
                        lane: bullet.lane,
                        depth,
                        angle: 0,
                    },
                    beasts::Reflected {
                        speed: if upgrades.laser { 0x24000 } else { 0x1c000 },
                    },
                    Transform::from_translation(transform.translation),
                    RenderLayers::layer(1),
                    Mesh3d(part),
                    MeshMaterial3d(flipper_assets.shaded.clone()),
                ));
                break;
            }
            if let Ok((_, mut spike)) = spikes.get_mut(entity) {
                game.score += 3;
                spike.length -= 3 * LANE_UNIT;
                if spike.length < 0 {
                    commands.entity(entity).despawn();
                    killed.insert(entity);
                    game.score += 3;
                }
                continue;
            }
            commands.entity(entity).despawn();
            killed.insert(entity);
            game.score += points;
            if let Ok((_, tanker, pose)) = tankers.get(entity) {
                tankers::open(
                    &mut commands,
                    &mut game,
                    &level,
                    &flipper_assets,
                    &shells,
                    tanker,
                );
                explosion::spawn_explosion(&mut commands, &bursts, pose.translation);
            }
            if let Some((lane, centre)) = centre {
                if upgrades.drop_due(level.powerups.kill_interval) {
                    powerups::spawn_drop(
                        &mut commands,
                        &mut upgrades,
                        &power_assets,
                        lane,
                        depth,
                        &level.web,
                    );
                } else {
                    explosion::spawn_explosion(&mut commands, &bursts, centre);
                }
            }
        }
    }
}

fn reset_wave(game: &mut Game, level: &Level) {
    game.remaining = level.flippers.count;
    game.spawn_timer = level.flippers.first_spawn_ticks - 1;
    game.status = Status::Entering;
    game.entry = Some(0);
    game.fire_timer = 112;
    game.tanker_timer = level
        .tankers
        .as_ref()
        .map_or(0, |g| g.first_spawn_ticks - 1);
    game.spiker_timer = level
        .spikers
        .as_ref()
        .map_or(0, |g| g.first_spawn_ticks - 1);
    game.beast_timer = level.beasts.as_ref().map_or(0, |g| g.first_spawn_ticks - 1);
}

#[allow(clippy::type_complexity)]
fn animate_capture(
    level: Res<Level>,
    mut game: ResMut<Game>,
    mut player: Single<&mut Transform, With<Glider>>,
    mut enemies: Query<(&mut Flipper, &mut Transform), Without<Glider>>,
    mut cameras: Query<&mut Transform, (With<Camera3d>, Without<Glider>, Without<Flipper>)>,
) {
    let Some(mut capture) = game.capture else {
        return;
    };
    if game.status != Status::Captured {
        return;
    }
    if capture.depth < level.web.depth {
        let step = (level.web.depth - capture.depth).min(2);
        capture.depth += step;
        player.translation.z -= f32::from(step) * WORLD_UNIT;
        if let Ok((mut flipper, mut transform)) = enemies.get_mut(capture.flipper) {
            flipper.depth += i32::from(step) * LANE_UNIT;
            transform.translation.z -= f32::from(step) * WORLD_UNIT;
        }
    } else {
        capture.camera_z += 8.0 * WORLD_UNIT;
        for mut camera in &mut cameras {
            camera.translation.z = capture.camera_z;
        }
        // snatch_it_away stops at vp_z <= -400; webz-80 is our rim's zero.
        if capture.camera_z >= RETREAT_END {
            game.status = Status::Respawning;
            game.capture = None;
            for mut camera in &mut cameras {
                camera.translation = level
                    .web
                    .camera_position(level.web.glider_transform(level.web.start()).translation);
            }
            return;
        }
    }
    game.capture = Some(capture);
}

#[allow(clippy::too_many_arguments)]
fn check_game(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut upgrades: ResMut<powerups::Upgrades>,
    bursts: Res<explosion::BurstAssets>,
    mut player: Single<(&mut Glider, &mut Weapon, &mut Transform, &mut Visibility)>,
    enemies: Query<(Entity, &Flipper, Option<&tankers::Released>)>,
    living: Query<(), With<Enemy>>,
    bullets: Query<Entity, With<Bullet>>,
    enemy_shots: Query<(Entity, &EnemyShot)>,
    pickups: Query<(Entity, &powerups::PowerUp)>,
    droids: Query<Entity, With<powerups::Droid>>,
) {
    if !matches!(game.status, Status::Playing | Status::Respawning) {
        return;
    }
    if game.entry.is_some_and(|tick| tick <= 256) {
        return;
    }
    let (glider, weapon, transform, visibility) = &mut *player;
    let grabber = (!weapon.firing)
        .then(|| {
            enemies.iter().find(|(_, flipper, released)| {
                released.is_none_or(|released| !released.burst)
                    && flipper.danger_lane(&level.web)
                        == Some(level.web.selected_lane(glider.progress) as usize)
            })
        })
        .flatten()
        .map(|(entity, _, _)| entity);
    if game.status == Status::Playing && game.invulnerable > 0 {
        game.invulnerable -= 1;
        **visibility = if game.invulnerable > 0 && (game.invulnerable / 6).is_multiple_of(2) {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    } else if game.status == Status::Respawning
        || (upgrades.jump_height == 0
            && (grabber.is_some()
                || enemy_shots.iter().any(|(_, shot)| {
                    shot.lane == level.web.selected_lane(glider.progress) as usize
                        && shot.depth <= 2 * LANE_UNIT
                })))
    {
        if game.status == Status::Playing
            && let Some(flipper) = grabber
        {
            game.status = Status::Captured;
            game.capture = Some(Capture {
                flipper,
                depth: 0,
                camera_z: level.web.camera_position(transform.translation).z,
            });
            glider.velocity = 0;
            **weapon = Weapon::default();
            **visibility = Visibility::Visible;
            return;
        }
        if game.status == Status::Playing {
            game.status = Status::ShotHit;
            game.death = Some(shot_death::Death::new(transform.translation, &level.web));
            glider.velocity = 0;
            **weapon = Weapon::default();
            **visibility = Visibility::Hidden;
            upgrades.lose_life();
            for entity in &bullets {
                commands.entity(entity).despawn();
            }
            for entity in &droids {
                commands.entity(entity).despawn();
            }
            for (entity, shot) in &enemy_shots {
                if shot.lane == level.web.selected_lane(glider.progress) as usize
                    && shot.depth <= 2 * LANE_UNIT
                {
                    let mut centre = transform.translation;
                    centre.z = -(shot.depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
                    commands.entity(entity).despawn();
                    explosion::spawn_explosion(&mut commands, &bursts, centre);
                }
            }
            return;
        }
        game.lives -= 1;
        if game.lives == 0 {
            // The final reset also removes actors while leaving the web visible.
            game.load = Some(LoadReason::Retry);
            game.status = Status::GameOver;
        } else {
            game.load = Some(LoadReason::Retry);
            game.status = Status::Entering;
            game.invulnerable = 120;
        }
        return;
    }
    if game.remaining == 0
        && living.is_empty()
        && enemy_shots.is_empty()
        && pickups.iter().all(|(_, pickup)| pickup.captured)
    {
        game.status = Status::Exiting;
        game.entry = None;
        game.exit = Some(exit::Exit::new(transform.translation, &level.web));
        **weapon = Weapon::default();
        **visibility = Visibility::Visible;
        for entity in &bullets {
            commands.entity(entity).despawn();
        }
    }
}

fn restart_game(keys: Res<ButtonInput<KeyCode>>, mut game: ResMut<Game>) {
    if keys.just_pressed(KeyCode::KeyR) {
        game.load = Some(LoadReason::Restart);
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn load_level(
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut level: ResMut<Level>,
    levels: Res<Levels>,
    mut upgrades: ResMut<powerups::Upgrades>,
    mut player: Single<(&mut Glider, &mut Weapon, &mut Transform, &mut Visibility)>,
    actors: Query<Entity, With<LevelActor>>,
    mut cameras: Query<&mut Transform, (With<Camera3d>, Without<Glider>)>,
    power_assets: Option<Res<powerups::PowerAssets>>,
    mut warp: Option<ResMut<stars::Warp>>,
) {
    let Some(reason) = game.load.take() else {
        return;
    };
    match reason {
        LoadReason::Restart => {
            *game = Game::default();
            *upgrades = powerups::Upgrades::default();
        }
        LoadReason::Retry => {
            upgrades.lose_life();
            game.invulnerable = if game.lives > 0 { 120 } else { 0 };
        }
        LoadReason::Advance => {
            game.invulnerable = 0;
            game.level_index = (game.level_index + 1) % levels.0.len();
            upgrades.land();
        }
    }
    for entity in &actors {
        commands.entity(entity).despawn();
    }
    *level = levels.0[game.level_index].clone();
    upgrades.begin_level();
    game.capture = None;
    game.death = None;
    game.exit = None;
    game.entry = None;
    if game.lives > 0 {
        reset_wave(&mut game, &level);
    } else {
        game.status = Status::GameOver;
    }
    if let Some(warp) = &mut warp {
        **warp = stars::Warp::default();
    }
    let (glider, weapon, transform, visibility) = &mut *player;
    glider.progress = level.web.start();
    glider.velocity = 0;
    **weapon = Weapon::default();
    **transform = level.web.glider_transform(level.web.start());
    **visibility = Visibility::Visible;
    for mut camera in &mut cameras {
        camera.translation = level.web.camera_position(transform.translation);
    }
    if upgrades.droid
        && let Some(assets) = power_assets
    {
        powerups::spawn_droid(&mut commands, &assets, &level.web);
    }
}

fn update_hud(game: Res<Game>, level: Res<Level>, mut hud: Single<&mut Text, With<GameHud>>) {
    if !game.is_changed() {
        return;
    }
    let status = match game.status {
        Status::Entering | Status::Playing => "",
        Status::Captured | Status::Respawning => "\nCAUGHT YOU",
        Status::ShotHit => "\nSHOT YOU!",
        Status::Spiked => "\nSPIKED!",
        Status::Exiting if level.spikers.is_some() => "\nLEVEL CLEAR\nAVOID THE SPIKES",
        Status::Exiting => "\nLEVEL CLEAR",
        Status::GameOver => "\nGAME OVER   R to restart",
    };
    let text = format!("{}\nLIVES {}{}", game.score, game.lives, status);
    if hud.0 != text {
        hud.0 = text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_loading_distinguishes_advancing_retrying_and_restarting() {
        let mut app = app();
        let mut second = Level::default();
        second.number = 2;
        second.web.start_lane = 3;
        second.web.depth = 180;
        second.flippers.count = 10;
        second.flippers.first_spawn_ticks = 11;
        app.world_mut().resource_mut::<Levels>().0.push(second);
        app.world_mut().resource_mut::<Game>().score = 9000;
        app.world_mut().resource_mut::<Game>().lives = 2;
        app.world_mut().resource_mut::<powerups::Upgrades>().laser = true;
        app.world_mut().resource_mut::<powerups::Upgrades>().droid = true;
        let old = enemy(&mut app, 7, 0, Motion::Pause(0));
        app.world_mut().resource_mut::<Game>().load = Some(LoadReason::Advance);
        ticks(&mut app, 1);
        assert!(app.world().get_entity(old).is_err());
        assert_eq!(app.world().resource::<Level>().number, 2);
        assert_eq!(
            (
                app.world().resource::<Game>().score,
                app.world().resource::<Game>().lives,
                app.world().resource::<Game>().remaining
            ),
            (9000, 2, 10)
        );
        assert_eq!(app.world().resource::<Game>().spawn_timer, 10);
        assert!(app.world().resource::<powerups::Upgrades>().laser);
        assert_eq!(
            app.world_mut()
                .query::<&powerups::Droid>()
                .iter(app.world())
                .count(),
            1
        );
        assert_eq!(
            app.world_mut()
                .query::<&Glider>()
                .single(app.world())
                .unwrap()
                .progress,
            3 * LANE_UNIT
        );
        app.world_mut().resource_mut::<Game>().load = Some(LoadReason::Retry);
        ticks(&mut app, 1);
        assert_eq!(app.world().resource::<Level>().number, 2);
        assert!(!app.world().resource::<powerups::Upgrades>().laser);
        assert_eq!(
            app.world_mut()
                .query::<&powerups::Droid>()
                .iter(app.world())
                .count(),
            0
        );
        assert_eq!(app.world().resource::<Game>().score, 9000);
        app.world_mut().resource_mut::<Game>().load = Some(LoadReason::Restart);
        ticks(&mut app, 1);
        assert_eq!(app.world().resource::<Level>().number, 1);
        assert_eq!(
            (
                app.world().resource::<Game>().score,
                app.world().resource::<Game>().lives
            ),
            (0, 3)
        );
    }

    #[test]
    fn approach_dots_stay_two_pixels_wide_and_brighten_with_proximity() {
        let camera = Transform::from_translation(Vec3::new(0.0, 0.0, 5.3));
        let projection = PerspectiveProjection {
            fov: 1.2,
            aspect_ratio: 1100.0 / 850.0,
            ..default()
        };
        let style = |depth: f32| {
            let position = Vec3::new(2.0, 1.0, -depth * WORLD_UNIT);
            let (width, intensity) = approach_dot_style(position, &camera, &projection);
            let screen_width = width * 384.0
                / (2.0
                    * (camera.translation.z - position.z)
                    * (projection.fov * 0.5).tan()
                    * projection.aspect_ratio);
            assert!((screen_width - 2.0).abs() < 1e-5);
            (width, intensity)
        };
        let far = style(460.0);
        let near = style(160.0);
        assert!(far.0 > near.0, "distant dots need larger world geometry");
        assert!(far.1 < near.1);
        assert!((60..=65).contains(&far.1));
        assert!((175..=185).contains(&near.1));
        assert_eq!(style(700.0).1, 0);
    }

    #[test]
    fn jumping_avoids_rim_contact_but_landing_restores_the_collision() {
        let mut app = app();
        app.add_systems(FixedUpdate, check_game);
        app.world_mut()
            .resource_mut::<powerups::Upgrades>()
            .jump_height = LANE_UNIT;
        app.world_mut().spawn(Flipper {
            lane: selected_lane(START_PROGRESS) as usize,
            depth: 0,
            motion: Motion::Pause(1),
            pause: 0,
        });
        app.world_mut().run_schedule(FixedUpdate);
        assert_eq!(app.world().resource::<Game>().lives, 3);
        app.world_mut()
            .resource_mut::<powerups::Upgrades>()
            .jump_height = 0;
        app.world_mut().run_schedule(FixedUpdate);
        assert!(app.world().resource::<Game>().status == Status::Captured);
        finish_capture(&mut app);
        assert_eq!(app.world().resource::<Game>().lives, 2);
    }

    pub(super) fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Game>()
            .init_resource::<Level>()
            .init_resource::<Levels>()
            .init_resource::<powerups::Upgrades>()
            .init_resource::<powerups::PowerAssets>()
            .init_resource::<FlipperAssets>()
            .init_resource::<tankers::TankerAssets>()
            .init_resource::<beasts::BeastAssets>()
            .init_resource::<explosion::BurstAssets>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(
                FixedUpdate,
                (entry::advance, animate_capture, shot_death::animate)
                    .chain()
                    .before(check_game),
            )
            .add_systems(
                FixedUpdate,
                load_level.after(check_game).after(restart_game),
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
        app
    }

    fn ticks(app: &mut App, count: usize) {
        for _ in 0..count {
            app.world_mut().run_schedule(FixedUpdate);
        }
    }

    fn finish_capture(app: &mut App) {
        assert!(app.world().resource::<Game>().status == Status::Captured);
        ticks(app, 130);
        assert!(app.world().resource::<Game>().capture.is_none());
    }

    fn enemy(app: &mut App, lane: usize, depth: i32, motion: Motion) -> Entity {
        let flipper = Flipper {
            lane,
            depth,
            motion,
            pause: 0,
        };
        let transform = flipper_transform(&flipper, &app.world().resource::<Level>().web);
        app.world_mut()
            .spawn((
                transform,
                flipper,
                Mesh3d::default(),
                MeshMaterial3d::<StandardMaterial>::default(),
            ))
            .id()
    }

    #[test]
    fn first_wave_spawns_25_flippers_with_original_timers_and_a_population_cap() {
        let mut app = app();
        app.add_systems(FixedUpdate, spawn_enemies);
        ticks(&mut app, 60);
        assert!(
            app.world_mut()
                .query::<&Flipper>()
                .iter(app.world())
                .next()
                .is_none()
        );
        ticks(&mut app, 1);
        assert_eq!(app.world().resource::<Game>().remaining, 24);
        ticks(&mut app, 121 * 15);
        assert_eq!(
            app.world_mut()
                .query::<&Flipper>()
                .iter(app.world())
                .count(),
            MAX_FLIPPERS
        );
        assert_eq!(app.world().resource::<Game>().remaining, 14);
        for _ in 0..14 {
            let entities: Vec<_> = app
                .world_mut()
                .query_filtered::<Entity, With<Flipper>>()
                .iter(app.world())
                .collect();
            for entity in entities {
                app.world_mut().despawn(entity);
            }
            ticks(&mut app, 121);
        }
        assert_eq!(app.world().resource::<Game>().remaining, 0);
    }

    #[test]
    fn flipper_approach_and_climb_use_original_fixed_point_speeds() {
        let mut app = app();
        app.add_systems(FixedUpdate, move_enemies);
        let id = enemy(&mut app, 0, REAR + 300 * LANE_UNIT, Motion::Approach);
        ticks(&mut app, 149);
        assert!(matches!(
            app.world().get::<Flipper>(id).unwrap().motion,
            Motion::Approach
        ));
        ticks(&mut app, 1);
        assert_eq!(app.world().get::<Flipper>(id).unwrap().depth, REAR - 0xa000);
        ticks(&mut app, 1);
        assert_eq!(
            app.world().get::<Flipper>(id).unwrap().depth,
            REAR - 2 * 0xa000
        );
        ticks(&mut app, 255);
        let flipper = app.world().get::<Flipper>(id).unwrap();
        assert_eq!(flipper.depth, 0);
        assert!(matches!(flipper.motion, Motion::Flip { .. }));
    }

    #[test]
    fn square_flippers_fold_along_sides_and_turn_correctly_at_corners() {
        for (from, to, duration) in [(0, 1, 32), (3, 4, 16), (4, 3, 16), (15, 0, 16)] {
            let mut app = app();
            let level = Level::parse(include_str!("../levels/02.toml"), "level2").unwrap();
            let mut flipper = Flipper {
                lane: from,
                depth: 0,
                motion: Motion::Pause(0),
                pause: 8,
            };
            start_flip(&mut flipper, to, &level.web);
            app.insert_resource(level.clone())
                .add_systems(FixedUpdate, move_enemies);
            let id = enemy(&mut app, to, 0, flipper.motion);
            ticks(&mut app, duration - 1);
            assert!(matches!(
                app.world().get::<Flipper>(id).unwrap().motion,
                Motion::Flip { .. }
            ));
            ticks(&mut app, 1);
            assert!(matches!(
                app.world().get::<Flipper>(id).unwrap().motion,
                Motion::Pause(_)
            ));
            let transform = app.world().get::<Transform>(id).unwrap();
            assert_eq!(
                transform.translation.truncate(),
                level
                    .web
                    .glider_transform(to as i32 * LANE_UNIT)
                    .translation
                    .truncate()
            );
            assert!(transform.rotation.dot(level.web.orientation(to)).abs() > 1.0 - 1e-6);
        }
    }

    #[test]
    fn edited_toml_values_control_population_and_climbing() {
        let source = include_str!("../levels/02.toml")
            .replace("count = 25", "count = 3")
            .replace("first_spawn_ticks = 51", "first_spawn_ticks = 2")
            .replace("spawn_interval_ticks = 101", "spawn_interval_ticks = 4")
            .replace("climb_speed = 0.625", "climb_speed = 1.5")
            .replace("max_active = 11", "max_active = 1");
        let level = Level::parse(&source, "tweaked-level.toml").unwrap();
        let mut app = app();
        reset_wave(&mut app.world_mut().resource_mut::<Game>(), &level);
        app.world_mut().resource_mut::<Game>().status = Status::Playing;
        app.world_mut().resource_mut::<Game>().entry = None;
        app.insert_resource(level)
            .add_systems(FixedUpdate, (spawn_enemies, move_enemies).chain());
        ticks(&mut app, 1);
        assert_eq!(app.world().resource::<Game>().remaining, 3);
        ticks(&mut app, 1);
        assert_eq!(app.world().resource::<Game>().remaining, 2);
        ticks(&mut app, 149);
        let (entity, flipper) = app
            .world_mut()
            .query::<(Entity, &Flipper)>()
            .single(app.world())
            .unwrap();
        assert_eq!(flipper.depth, REAR - 0x18000);
        assert_eq!(app.world().resource::<Game>().remaining, 2);
        app.world_mut().despawn(entity);
        ticks(&mut app, 1);
        assert_eq!(app.world().resource::<Game>().remaining, 1);
    }

    #[test]
    fn square_wave_uses_its_own_initial_and_repeat_spawn_intervals() {
        let mut app = app();
        let level = Level::parse(include_str!("../levels/02.toml"), "level2").unwrap();
        reset_wave(&mut app.world_mut().resource_mut::<Game>(), &level);
        app.world_mut().resource_mut::<Game>().status = Status::Playing;
        app.world_mut().resource_mut::<Game>().entry = None;
        app.insert_resource(level)
            .add_systems(FixedUpdate, spawn_enemies);
        ticks(&mut app, 50);
        assert_eq!(app.world().resource::<Game>().remaining, 25);
        ticks(&mut app, 1);
        assert_eq!(app.world().resource::<Game>().remaining, 24);
        ticks(&mut app, 100);
        assert_eq!(app.world().resource::<Game>().remaining, 24);
        ticks(&mut app, 1);
        assert_eq!(app.world().resource::<Game>().remaining, 23);
    }

    #[test]
    fn rim_flips_take_28_ticks_seek_the_player_and_wrap() {
        let mut app = app();
        app.add_systems(FixedUpdate, move_enemies);
        let mut flipper = Flipper {
            lane: 15,
            depth: 0,
            motion: Motion::Pause(0),
            pause: 0,
        };
        start_flip(&mut flipper, 0, &Level::default().web);
        assert_eq!(flipper.lane, 0);
        assert_eq!(flipper.hit_lane(&Level::default().web), None);
        assert_eq!(flipper.danger_lane(&Level::default().web), Some(15));
        let id = app
            .world_mut()
            .spawn((
                flipper_transform(&flipper, &Level::default().web),
                flipper,
                Mesh3d::default(),
                MeshMaterial3d::<StandardMaterial>::default(),
            ))
            .id();
        ticks(&mut app, 15);
        assert_eq!(
            app.world()
                .get::<Flipper>(id)
                .unwrap()
                .hit_lane(&Level::default().web),
            Some(0)
        );
        ticks(&mut app, 13);
        assert!(matches!(
            app.world().get::<Flipper>(id).unwrap().motion,
            Motion::Pause(0)
        ));
        let mut reverse = Flipper {
            lane: 0,
            depth: 0,
            motion: Motion::Pause(0),
            pause: 4,
        };
        start_flip(&mut reverse, 15, &Level::default().web);
        assert_eq!(reverse.lane, 15);
        assert!(matches!(reverse.motion, Motion::Flip { direction: -1, .. }));
    }

    #[test]
    fn active_fire_covers_the_normal_shot_delay_but_releasing_fire_allows_a_grab() {
        for laser in [false, true] {
            for release in [false, true] {
                let mut app = app();
                app.init_resource::<ShotAssets>().add_systems(
                    FixedUpdate,
                    (
                        move_bullets,
                        fire_bullets,
                        move_enemies,
                        resolve_hits,
                        check_game,
                    )
                        .chain(),
                );
                app.world_mut().resource_mut::<powerups::Upgrades>().laser = laser;
                app.world_mut()
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .press(KeyCode::Enter);
                ticks(&mut app, 2);
                let flipper = enemy(
                    &mut app,
                    7,
                    0,
                    Motion::Flip {
                        from: 6,
                        direction: 1,
                        angle: 56,
                    },
                );
                ticks(&mut app, 1);
                assert_eq!(app.world().resource::<Game>().lives, 3);
                assert!(
                    app.world().get_entity(flipper).is_ok(),
                    "the latest shot is already beyond the rim collision range"
                );
                if release {
                    app.world_mut()
                        .resource_mut::<ButtonInput<KeyCode>>()
                        .release(KeyCode::Enter);
                }
                ticks(&mut app, if laser { 1 } else { 2 });
                assert_eq!(app.world().resource::<Game>().lives, 3);
                assert!(
                    app.world().resource::<Game>().status
                        == if release {
                            Status::Captured
                        } else {
                            Status::Playing
                        }
                );
                assert_eq!(
                    app.world().resource::<Game>().score,
                    if release { 0 } else { 150 }
                );
            }
        }
    }

    #[test]
    fn a_full_magazine_allows_a_grab_even_with_enter_held() {
        let mut app = app();
        app.init_resource::<ShotAssets>().add_systems(
            FixedUpdate,
            (
                move_bullets,
                fire_bullets,
                move_enemies,
                resolve_hits,
                check_game,
            )
                .chain(),
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        for _ in 0..MAX_BULLETS {
            app.world_mut()
                .spawn((Bullet { lane: 0 }, Transform::from_xyz(0.0, 0.0, -10.0)));
        }
        enemy(
            &mut app,
            7,
            0,
            Motion::Flip {
                from: 6,
                direction: 1,
                angle: 56,
            },
        );
        ticks(&mut app, 1);
        assert!(app.world().resource::<Game>().status == Status::Captured);
    }

    #[test]
    fn holding_fire_without_moving_cannot_clear_the_first_wave() {
        for laser in [false, true] {
            let mut app = app();
            app.world_mut().resource_mut::<powerups::Upgrades>().laser = laser;
            app.init_resource::<ShotAssets>().add_systems(
                FixedUpdate,
                (
                    move_bullets,
                    fire_bullets.run_if(playing),
                    spawn_enemies,
                    move_enemies,
                    move_enemy_shots,
                    resolve_hits,
                    fire_enemy_shots,
                    powerups::advance_powerups,
                    check_game,
                )
                    .chain(),
            );
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Enter);
            for _ in 0..15_000 {
                ticks(&mut app, 1);
                if app.world().resource::<Game>().lives < 3 {
                    break;
                }
                assert!(
                    app.world().resource::<Game>().status != Status::Exiting,
                    "stationary held fire cleared the wave"
                );
            }
            assert!(
                app.world().resource::<Game>().lives < 3,
                "stationary held fire survived the entire wave, laser={laser}"
            );
        }
    }

    #[test]
    fn firing_destroys_an_incoming_flipper_before_it_can_grab_the_player() {
        for laser in [false, true] {
            let mut app = app();
            app.init_resource::<ShotAssets>().add_systems(
                FixedUpdate,
                (
                    move_bullets,
                    fire_bullets,
                    move_enemies,
                    resolve_hits,
                    check_game,
                )
                    .chain(),
            );
            app.world_mut().resource_mut::<powerups::Upgrades>().laser = laser;
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Enter);
            enemy(
                &mut app,
                7,
                0,
                Motion::Flip {
                    from: 6,
                    direction: 1,
                    angle: 56,
                },
            );
            ticks(&mut app, 1);
            assert_eq!(app.world().resource::<Game>().lives, 3);
            assert_eq!(app.world().resource::<Game>().score, 150);
        }
    }

    #[test]
    fn a_pause_in_fire_lets_an_incoming_flipper_grab_the_player() {
        let mut app = app();
        app.init_resource::<ShotAssets>().add_systems(
            FixedUpdate,
            (
                move_bullets,
                fire_bullets,
                move_enemies,
                resolve_hits,
                check_game,
            )
                .chain(),
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        enemy(
            &mut app,
            7,
            0,
            Motion::Flip {
                from: 6,
                direction: 1,
                angle: 0,
            },
        );
        ticks(&mut app, 8);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Enter);
        ticks(&mut app, 24);
        assert!(app.world().resource::<Game>().status == Status::Captured);
        assert_eq!(app.world().resource::<Game>().score, 0);
    }

    #[test]
    fn shots_at_the_rim_leave_a_gap_between_normal_fire_intervals() {
        let mut app = app();
        app.add_systems(FixedUpdate, resolve_hits);
        let flipper = enemy(&mut app, 7, 0, Motion::Pause(4));
        for depth in [6.5, 6.5 + 3.5 * f32::from(FIRE_INTERVAL)] {
            app.world_mut().spawn((
                Bullet { lane: 7 },
                Transform::from_xyz(0.0, 0.0, 0.06 - depth * WORLD_UNIT),
            ));
        }
        ticks(&mut app, 1);
        assert!(app.world().get_entity(flipper).is_ok());
        app.world_mut().spawn((
            Bullet { lane: 7 },
            Transform::from_xyz(0.0, 0.0, 0.06 - 5.5 * WORLD_UNIT),
        ));
        ticks(&mut app, 1);
        assert!(app.world().get_entity(flipper).is_err());
    }

    #[test]
    fn particle_shots_pass_through_multiple_enemies() {
        let mut app = app();
        app.add_systems(FixedUpdate, resolve_hits);
        let near = enemy(&mut app, 3, 60 * LANE_UNIT, Motion::Climb);
        let far = enemy(&mut app, 3, 64 * LANE_UNIT, Motion::Climb);
        let shot = app
            .world_mut()
            .spawn((
                Bullet { lane: 3 },
                ParticleShot::default(),
                Transform::from_xyz(0.0, 0.0, -10.3),
            ))
            .id();
        ticks(&mut app, 1);
        assert!(app.world().get_entity(near).is_err());
        assert!(app.world().get_entity(far).is_err());
        assert!(app.world().get_entity(shot).is_ok());
        assert_eq!(app.world().resource::<Game>().score, 300);
    }

    #[test]
    fn shots_hit_only_their_lane_and_kill_one_nearest_enemy_for_150_points() {
        let mut app = app();
        app.add_systems(FixedUpdate, resolve_hits);
        let near = enemy(&mut app, 3, 60 * LANE_UNIT, Motion::Climb);
        let far = enemy(&mut app, 3, 64 * LANE_UNIT, Motion::Climb);
        let other = enemy(&mut app, 4, 60 * LANE_UNIT, Motion::Climb);
        let bullet = app
            .world_mut()
            .spawn((Bullet { lane: 3 }, Transform::from_xyz(0.0, 0.0, -10.3)))
            .id();
        ticks(&mut app, 1);
        assert!(app.world().get_entity(near).is_err());
        assert!(app.world().get_entity(bullet).is_err());
        assert!(app.world().get_entity(far).is_ok());
        assert!(app.world().get_entity(other).is_ok());
        assert_eq!(app.world().resource::<Game>().score, 150);
    }

    #[test]
    fn duplicate_shots_cannot_award_the_same_kill_twice() {
        let mut app = app();
        app.add_systems(FixedUpdate, resolve_hits);
        enemy(&mut app, 2, 60 * LANE_UNIT, Motion::Climb);
        for _ in 0..2 {
            app.world_mut()
                .spawn((Bullet { lane: 2 }, Transform::from_xyz(0.0, 0.0, -10.0)));
        }
        ticks(&mut app, 1);
        assert_eq!(app.world().resource::<Game>().score, 150);
        assert_eq!(
            app.world_mut()
                .query::<&powerups::PowerUp>()
                .iter(app.world())
                .count(),
            1
        );
        assert!(
            !app.world().resource::<powerups::Upgrades>().laser,
            "a drop must be collected first"
        );
        assert_eq!(
            app.world_mut().query::<&Bullet>().iter(app.world()).count(),
            1
        );
    }

    #[test]
    fn capture_preserves_both_poses_then_retreats_the_camera_before_losing_a_life() {
        let mut app = app();
        app.init_resource::<ShotAssets>().add_systems(
            FixedUpdate,
            (
                move_glider.run_if(playing),
                powerups::update_jump,
                follow_glider.run_if(playing),
                fire_bullets.run_if(playing),
                move_enemies,
                check_game,
            )
                .chain(),
        );
        let player = app
            .world_mut()
            .query_filtered::<Entity, With<Glider>>()
            .single(app.world())
            .unwrap();
        let camera_start = Transform::from_translation(camera_position(
            glider_transform(START_PROGRESS).translation,
        ));
        let cameras = [0, 1].map(|_| {
            app.world_mut()
                .spawn((Camera3d::default(), camera_start))
                .id()
        });
        let captor = enemy(
            &mut app,
            7,
            0,
            Motion::Flip {
                from: 6,
                direction: 1,
                angle: 56,
            },
        );
        let other = enemy(&mut app, 2, 60 * LANE_UNIT, Motion::Climb);
        ticks(&mut app, 1);
        assert!(app.world().resource::<Game>().status == Status::Captured);
        let claw_start = *app.world().get::<Transform>(player).unwrap();
        let flipper_start = *app.world().get::<Transform>(captor).unwrap();
        let other_start = *app.world().get::<Transform>(other).unwrap();
        app.world_mut()
            .resource_mut::<powerups::Upgrades>()
            .jump_enabled = true;
        for key in [KeyCode::KeyA, KeyCode::Enter, KeyCode::KeyJ] {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
        }
        ticks(&mut app, 79);
        for (entity, start) in [(player, claw_start), (captor, flipper_start)] {
            let transform = app.world().get::<Transform>(entity).unwrap();
            assert_eq!(
                transform.translation.truncate(),
                start.translation.truncate()
            );
            assert_eq!(transform.rotation, start.rotation);
            assert!(
                (transform.translation.z - (start.translation.z - 158.0 * WORLD_UNIT)).abs() < 1e-4
            );
        }
        assert_eq!(*app.world().get::<Transform>(other).unwrap(), other_start);
        assert_eq!(
            app.world().get::<Glider>(player).unwrap().progress,
            START_PROGRESS
        );
        assert_eq!(app.world().resource::<powerups::Upgrades>().jump_height, 0);
        assert_eq!(
            app.world_mut().query::<&Bullet>().iter(app.world()).count(),
            0
        );
        ticks(&mut app, 1);
        assert_eq!(app.world().get::<Flipper>(captor).unwrap().depth, REAR);
        assert_eq!(app.world().resource::<Game>().lives, 3);
        for camera in cameras {
            assert_eq!(*app.world().get::<Transform>(camera).unwrap(), camera_start);
        }
        ticks(&mut app, 1);
        for camera in cameras {
            assert!(
                (app.world().get::<Transform>(camera).unwrap().translation.z
                    - camera_start.translation.z
                    - 8.0 * WORLD_UNIT)
                    .abs()
                    < 1e-5
            );
        }
        ticks(&mut app, 48);
        assert_eq!(app.world().resource::<Game>().lives, 3);
        ticks(&mut app, 1);
        let game = app.world().resource::<Game>();
        assert!(game.status == Status::Entering);
        assert_eq!(game.lives, 2);
        assert_eq!(game.invulnerable, 120);
        assert!(game.capture.is_none());
        assert!(app.world().get_entity(captor).is_err());
        assert!(app.world().get_entity(other).is_err());
        assert_eq!(
            *app.world().get::<Transform>(player).unwrap(),
            glider_transform(START_PROGRESS)
        );
        for camera in cameras {
            assert_eq!(*app.world().get::<Transform>(camera).unwrap(), camera_start);
        }
    }

    #[test]
    fn restarting_during_capture_restores_the_camera_and_cancels_the_animation() {
        let mut app = app();
        app.add_systems(FixedUpdate, (check_game, restart_game).chain());
        let camera_start = camera_position(glider_transform(START_PROGRESS).translation);
        let camera = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                Transform::from_translation(camera_start),
            ))
            .id();
        enemy(&mut app, 7, 0, Motion::Pause(0));
        ticks(&mut app, 82);
        assert!(app.world().resource::<Game>().status == Status::Captured);
        assert!(app.world().get::<Transform>(camera).unwrap().translation.z > camera_start.z);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        ticks(&mut app, 1);
        assert!(app.world().resource::<Game>().status == Status::Entering);
        assert!(app.world().resource::<Game>().capture.is_none());
        assert_eq!(app.world().resource::<Game>().lives, 3);
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            camera_start
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyR);
        ticks(&mut app, 150);
        assert_eq!(app.world().resource::<Game>().lives, 3);
    }

    #[test]
    fn rim_contact_costs_a_life_resets_the_wave_and_grants_a_grace_period() {
        let mut app = app();
        app.add_systems(FixedUpdate, check_game);
        app.world_mut().resource_mut::<Game>().score = 300;
        enemy(&mut app, 7, 0, Motion::Pause(4));
        ticks(&mut app, 1);
        assert_eq!(app.world().resource::<Game>().lives, 3);
        finish_capture(&mut app);
        let game = app.world().resource::<Game>();
        assert_eq!(game.lives, 2);
        assert_eq!(game.score, 300);
        assert_eq!(game.invulnerable, 120);
        ticks(&mut app, 256);
        enemy(&mut app, 7, 0, Motion::Pause(4));
        ticks(&mut app, 120);
        assert_eq!(app.world().resource::<Game>().lives, 2);
        ticks(&mut app, 1);
        finish_capture(&mut app);
        assert_eq!(app.world().resource::<Game>().lives, 1);
        ticks(&mut app, 256);
        app.world_mut().resource_mut::<Game>().invulnerable = 0;
        enemy(&mut app, 7, 0, Motion::Pause(4));
        ticks(&mut app, 1);
        finish_capture(&mut app);
        assert!(app.world().resource::<Game>().status == Status::GameOver);
    }

    #[test]
    fn completion_requires_no_remaining_spawns_and_no_live_enemies() {
        let mut app = app();
        app.add_systems(FixedUpdate, check_game);
        app.world_mut().resource_mut::<Game>().remaining = 0;
        let id = enemy(&mut app, 1, REAR, Motion::Climb);
        ticks(&mut app, 1);
        assert!(app.world().resource::<Game>().status == Status::Playing);
        app.world_mut().despawn(id);
        powerups::spawn_drop(
            &mut app.world_mut().commands(),
            &mut powerups::Upgrades::default(),
            &powerups::PowerAssets::default(),
            1,
            REAR,
            &Level::default().web,
        );
        app.world_mut().flush();
        ticks(&mut app, 1);
        assert!(app.world().resource::<Game>().status == Status::Playing);
        let pickup = app
            .world_mut()
            .query_filtered::<Entity, With<powerups::PowerUp>>()
            .single(app.world())
            .unwrap();
        app.world_mut().despawn(pickup);
        ticks(&mut app, 1);
        assert!(app.world().resource::<Game>().status == Status::Exiting);
    }

    #[test]
    fn restart_clears_enemies_shots_score_and_game_over() {
        let mut app = app();
        app.add_systems(FixedUpdate, restart_game);
        app.world_mut().resource_mut::<Game>().status = Status::GameOver;
        app.world_mut().resource_mut::<Game>().score = 150;
        enemy(&mut app, 3, REAR, Motion::Climb);
        app.world_mut().resource_mut::<powerups::Upgrades>().laser = true;
        powerups::spawn_drop(
            &mut app.world_mut().commands(),
            &mut powerups::Upgrades::default(),
            &powerups::PowerAssets::default(),
            3,
            REAR,
            &Level::default().web,
        );
        app.world_mut().flush();
        app.world_mut().spawn(Bullet { lane: 2 });
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        ticks(&mut app, 1);
        assert_eq!(app.world().resource::<Game>().score, 0);
        assert_eq!(app.world().resource::<Game>().lives, 3);
        assert!(!app.world().resource::<powerups::Upgrades>().laser);
        assert_eq!(
            app.world_mut()
                .query::<&powerups::PowerUp>()
                .iter(app.world())
                .count(),
            0
        );
        assert!(app.world().resource::<Game>().status == Status::Entering);
        assert!(
            app.world_mut()
                .query::<&Flipper>()
                .iter(app.world())
                .next()
                .is_none()
        );
        assert!(
            app.world_mut()
                .query::<&Bullet>()
                .iter(app.world())
                .next()
                .is_none()
        );
    }

    #[test]
    fn flipper_geometry_and_colors_match_the_measured_original_render() {
        let mesh = flipper_mesh([240, 0, 0]);
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|attribute| attribute.as_float3())
            .expect("flipper mesh needs positions");
        let colors = match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
            Some(bevy::mesh::VertexAttributeValues::Float32x4(values)) => values,
            other => panic!("flipper mesh needs colours, got {other:?}"),
        };
        assert_eq!(positions.len(), 12, "four triangles");
        assert_eq!(colors.len(), 12);

        let red = |index: usize| colors[index][0];
        assert!(
            colors
                .iter()
                .all(|color| color[1] == 0.0 && color[2] == 0.0),
            "the original renders no green or blue on a Flipper"
        );

        // Two brighter and two dimmer facets, from the source intensities.
        let shades: Vec<f32> = (0..4).map(|face| red(face * 3)).collect();
        assert_eq!(shades[0], shades[2]);
        assert_eq!(shades[1], shades[3]);
        assert!(shades[0] > shades[1]);
        let brightest = LinearRgba::from(Color::srgb_u8(BRIGHTEST_RED, 0, 0)).to_f32_array()[0];
        assert!((shades[0] - brightest).abs() < 1e-6);

        // Each face is a flat facet, so its three vertices share one colour.
        for face in 0..4 {
            let first = red(face * 3);
            assert_eq!(red(face * 3 + 1), first);
            assert_eq!(red(face * 3 + 2), first);
        }

        // The four tips sit on the corners of a box twice as wide as it is tall,
        // which is what makes the X read as an elongated cross instead of
        // closing towards a plus sign. See the reference notes in the README.
        let min = |index: usize| positions.iter().map(|p| p[index]).fold(f32::MAX, f32::min);
        let max = |index: usize| positions.iter().map(|p| p[index]).fold(f32::MIN, f32::max);
        let (width, height) = (max(0) - min(0), max(1) - min(1));
        assert!(
            (width / height - 2.0).abs() < 0.01,
            "tip box should be twice as wide as tall, got {width} by {height}"
        );
    }

    #[test]
    fn flippers_fire_one_enemy_shot_at_a_time_with_original_speed() {
        let mut app = app();
        app.add_systems(FixedUpdate, (move_enemy_shots, fire_enemy_shots).chain());
        enemy(&mut app, 1, REAR, Motion::Climb);
        ticks(&mut app, 112);
        assert!(
            app.world_mut()
                .query::<&EnemyShot>()
                .iter(app.world())
                .next()
                .is_none()
        );
        ticks(&mut app, 1);
        let shot = app
            .world_mut()
            .query_filtered::<Entity, With<EnemyShot>>()
            .single(app.world())
            .unwrap();
        ticks(&mut app, 1);
        assert_eq!(
            app.world().get::<EnemyShot>(shot).unwrap().depth,
            REAR - 0x18000
        );
        assert_eq!(app.world().get::<EnemyShot>(shot).unwrap().angle, 4);
        for _ in 0..100 {
            ticks(&mut app, 1);
            assert!(
                app.world_mut()
                    .query::<&EnemyShot>()
                    .iter(app.world())
                    .count()
                    <= 1
            );
        }
    }

    #[test]
    fn enemy_shots_are_interceptable_and_do_not_award_flipper_points() {
        let mut app = app();
        app.add_systems(FixedUpdate, resolve_hits);
        let shot = app
            .world_mut()
            .spawn(EnemyShot {
                lane: 1,
                depth: 60 * LANE_UNIT,
                angle: 0,
            })
            .id();
        app.world_mut()
            .spawn((Bullet { lane: 1 }, Transform::from_xyz(0., 0., -10.)));
        ticks(&mut app, 1);
        assert!(app.world().get_entity(shot).is_err());
        assert_eq!(app.world().resource::<Game>().score, 0);
    }

    #[test]
    fn enemy_shot_reaching_the_player_lane_costs_a_life() {
        let mut app = app();
        app.add_systems(FixedUpdate, check_game);
        app.world_mut().spawn(EnemyShot {
            lane: 7,
            depth: LANE_UNIT,
            angle: 0,
        });
        ticks(&mut app, 1);
        assert!(app.world().resource::<Game>().status == Status::ShotHit);
        assert_eq!(app.world().resource::<Game>().lives, 3);
        assert!(
            app.world_mut()
                .query::<&EnemyShot>()
                .iter(app.world())
                .next()
                .is_none()
        );
        ticks(&mut app, 163);
        assert_eq!(app.world().resource::<Game>().lives, 2);
        assert!(app.world().resource::<Game>().status == Status::Entering);
    }
}
