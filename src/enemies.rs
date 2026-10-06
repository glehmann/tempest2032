use std::{collections::HashSet, f32::consts::TAU};

use super::*;

use crate::explosion;

const WORLD_UNIT: f32 = FRONT_RADIUS / 24.0;
const REAR: i32 = 160 * LANE_UNIT;
const FLIPPER_SPEED: i32 = 0xa000;
const FLIP_SPEED: u8 = 4;
const FLIP_ANGLE: u8 = 112;
const MAX_FLIPPERS: usize = 11;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Playing,
    Clear,
    GameOver,
}

#[derive(Resource)]
struct Game {
    score: u32,
    lives: u8,
    status: Status,
    remaining: u8,
    spawn_timer: u16,
    invulnerable: u16,
    random: u32,
    fire_timer: u8,
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
        angle: u8,
    },
    Pause(u8),
}

#[derive(Component, Debug)]
struct Flipper {
    lane: usize,
    depth: i32,
    motion: Motion,
    pause: u8,
}

#[derive(Component)]
struct EnemyShot {
    lane: usize,
    depth: i32,
    angle: u8,
}

impl Flipper {
    fn hit_lane(&self) -> Option<usize> {
        match self.motion {
            Motion::Approach => None,
            Motion::Flip { angle, .. } if FLIP_ANGLE - angle > 52 => None,
            _ => Some(self.lane),
        }
    }

    fn danger_lane(&self) -> Option<usize> {
        match self.motion {
            Motion::Pause(_) => Some(self.lane),
            Motion::Flip { from, angle, .. } if FLIP_ANGLE - angle > 52 => Some(from),
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
    app.init_resource::<Game>()
        .add_systems(
            FixedUpdate,
            (
                move_glider.run_if(playing),
                animate_glider,
                update_lane,
                follow_glider,
                move_bullets,
                fire_bullets.run_if(playing),
                spawn_enemies,
                move_enemies,
                move_enemy_shots,
            )
                .chain(),
        )
        .add_systems(FixedUpdate, (resolve_hits, fire_enemy_shots).chain())
        .add_systems(FixedUpdate, explosion::age_explosions)
        .add_systems(FixedUpdate, check_game)
        .add_systems(
            Update,
            (fit_camera, quit_on_escape, restart_game, update_hud).chain(),
        )
        .add_systems(Update, draw_approaching_flippers.after(fit_camera));
}

fn playing(game: Res<Game>) -> bool {
    game.status == Status::Playing
}

const BRIGHTEST_RED: u8 = 240;

fn flipper_mesh() -> Mesh {
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
        let color = Color::srgb_u8((red * u16::from(BRIGHTEST_RED) / 255) as u8, 0, 0);
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
) {
    explosion::setup(commands, meshes, materials);
    commands.insert_resource(FlipperAssets {
        body: meshes.add(flipper_mesh()),
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
                    base_color: Color::srgb_u8((254 * intensity / 256) as u8, 0, 0),
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

fn orientation(lane: usize) -> Quat {
    Quat::from_rotation_z(-(lane as f32) * TAU / LANES as f32)
}

fn flipper_transform(flipper: &Flipper) -> Transform {
    let z = -(flipper.depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
    if let Motion::Flip {
        from,
        direction,
        angle,
    } = flipper.motion
    {
        let pivot = rim_vertex(
            (from as i32 + i32::from(direction > 0)) as f32,
            FRONT_RADIUS,
            z,
        );
        let rotation = orientation(from)
            * Quat::from_rotation_z(direction as f32 * angle as f32 * TAU / 256.0);
        return Transform::from_translation(
            pivot + rotation * Vec3::X * (-direction as f32 * 9.0 * 0.075),
        )
        .with_rotation(rotation);
    }
    let mut transform = glider_transform(flipper.lane as i32 * LANE_UNIT);
    transform.translation.z = z;
    transform.rotation = orientation(flipper.lane);
    transform
}

fn spawn_enemies(
    mut commands: Commands,
    mut game: ResMut<Game>,
    assets: Res<FlipperAssets>,
    enemies: Query<&Flipper>,
) {
    if game.status != Status::Playing || game.remaining == 0 {
        return;
    }
    if game.spawn_timer > 0 {
        game.spawn_timer -= 1;
        return;
    }
    if enemies.iter().count() >= MAX_FLIPPERS {
        return;
    }
    game.random = game.random.wrapping_mul(1664525).wrapping_add(1013904223);
    let flipper = Flipper {
        lane: (game.random >> 16) as usize % LANES,
        depth: REAR + 300 * LANE_UNIT,
        motion: Motion::Approach,
        pause: 0,
    };
    commands.spawn((
        flipper_transform(&flipper),
        flipper,
        RenderLayers::layer(1),
        Mesh3d(assets.point.clone()),
        MeshMaterial3d(assets.point_shades.first().cloned().unwrap_or_default()),
    ));
    game.remaining -= 1;
    game.spawn_timer = 120;
}

fn start_flip(flipper: &mut Flipper, player_lane: usize) {
    if flipper.lane == player_lane {
        flipper.motion = Motion::Pause(flipper.pause);
        return;
    }
    let clockwise = (player_lane + LANES - flipper.lane) % LANES;
    let direction = if clockwise < LANES / 2 { 1 } else { -1 };
    let from = flipper.lane;
    flipper.lane = (from as i32 + direction).rem_euclid(LANES as i32) as usize;
    flipper.motion = Motion::Flip {
        from,
        direction,
        angle: 0,
    };
}

fn move_enemies(
    game: Res<Game>,
    player: Single<&Glider>,
    assets: Res<FlipperAssets>,
    mut enemies: Query<(
        &mut Flipper,
        &mut Transform,
        &mut Mesh3d,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    if game.status != Status::Playing {
        return;
    }
    let player_lane = selected_lane(player.progress) as usize;
    for (mut flipper, mut transform, mut mesh, mut material) in &mut enemies {
        match flipper.motion {
            Motion::Approach => {
                flipper.depth -= 2 * LANE_UNIT;
                if flipper.depth <= REAR {
                    flipper.depth = REAR - FLIPPER_SPEED;
                    flipper.motion = Motion::Climb;
                    mesh.0 = assets.body.clone();
                    material.0 = assets.shaded.clone();
                }
            }
            Motion::Climb => {
                flipper.depth = (flipper.depth - FLIPPER_SPEED).max(0);
                if flipper.depth == 0 {
                    start_flip(&mut flipper, player_lane);
                }
            }
            Motion::Flip {
                from,
                direction,
                angle,
            } => {
                let angle = angle + FLIP_SPEED;
                flipper.motion = if angle >= FLIP_ANGLE {
                    Motion::Pause(flipper.pause)
                } else {
                    Motion::Flip {
                        from,
                        direction,
                        angle,
                    }
                };
            }
            Motion::Pause(0) => {
                flipper.pause = 4;
                start_flip(&mut flipper, player_lane);
            }
            Motion::Pause(ticks) => flipper.motion = Motion::Pause(ticks - 1),
        }
        *transform = flipper_transform(&flipper);
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
    mut commands: Commands,
    mut game: ResMut<Game>,
    assets: Res<FlipperAssets>,
    enemies: Query<&Flipper>,
    shots: Query<&EnemyShot>,
) {
    if game.status != Status::Playing {
        return;
    }
    for flipper in &enemies {
        if !matches!(flipper.motion, Motion::Climb) {
            continue;
        }
        if game.fire_timer > 0 {
            game.fire_timer -= 1;
            continue;
        }
        game.fire_timer = 112;
        if flipper.depth <= 60 * LANE_UNIT || !shots.is_empty() {
            continue;
        }
        let shot = EnemyShot {
            lane: flipper.lane,
            depth: flipper.depth,
            angle: 0,
        };
        commands.spawn((
            Transform::from_translation(flipper_transform(flipper).translation),
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
    mut shots: Query<(Entity, &mut EnemyShot, &mut Transform)>,
) {
    if game.status != Status::Playing {
        return;
    }
    for (entity, mut shot, mut transform) in &mut shots {
        shot.depth -= 0x18000;
        shot.angle = shot.angle.wrapping_add(4);
        transform.translation.z = -(shot.depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
        transform.rotation = Quat::from_rotation_z(-(shot.angle as f32) * TAU / 256.0);
        if shot.depth < 0 {
            commands.entity(entity).despawn();
        }
    }
}

fn resolve_hits(
    mut commands: Commands,
    mut game: ResMut<Game>,
    bursts: Res<explosion::BurstAssets>,
    bullets: Query<(Entity, &Bullet, &Transform)>,
    enemies: Query<(Entity, &Flipper, &Transform)>,
    enemy_shots: Query<(Entity, &EnemyShot)>,
) {
    if game.status != Status::Playing {
        return;
    }
    let mut killed = HashSet::new();
    for (shot, bullet, transform) in &bullets {
        let intersects = |lane, depth: i32| {
            let z = -(depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
            lane == bullet.lane
                && z >= transform.translation.z - WORLD_UNIT * 6.0
                && z <= bullet.previous_z + WORLD_UNIT * 6.0
        };
        let hit = enemies
            .iter()
            .filter(|(entity, flipper, _)| {
                !killed.contains(entity)
                    && flipper.hit_lane() == Some(bullet.lane)
                    && intersects(flipper.lane, flipper.depth)
            })
            .map(|(entity, flipper, transform)| {
                (entity, flipper.depth, 150, Some(transform.translation))
            })
            .chain(
                enemy_shots
                    .iter()
                    .filter(|(entity, shot)| {
                        !killed.contains(entity) && intersects(shot.lane, shot.depth)
                    })
                    .map(|(entity, shot)| (entity, shot.depth, 0, None::<Vec3>)),
            )
            .min_by_key(|(_, depth, _, _)| *depth);
        if let Some((entity, _, points, centre)) = hit {
            commands.entity(shot).despawn();
            commands.entity(entity).despawn();
            killed.insert(entity);
            game.score += points;
            if let Some(centre) = centre {
                explosion::spawn_explosion(&mut commands, &bursts, centre);
            }
        }
    }
}

fn reset_wave(game: &mut Game) {
    game.remaining = 25;
    game.spawn_timer = 60;
    game.status = Status::Playing;
    game.fire_timer = 112;
}

fn check_game(
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut player: Single<(&mut Glider, &mut Weapon, &mut Transform, &mut Visibility)>,
    enemies: Query<(Entity, &Flipper)>,
    bullets: Query<Entity, With<Bullet>>,
    enemy_shots: Query<(Entity, &EnemyShot)>,
) {
    if game.status != Status::Playing {
        return;
    }
    let (glider, weapon, transform, visibility) = &mut *player;
    if game.invulnerable > 0 {
        game.invulnerable -= 1;
        **visibility = if game.invulnerable > 0 && (game.invulnerable / 6).is_multiple_of(2) {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    } else if enemies
        .iter()
        .any(|(_, flipper)| flipper.danger_lane() == Some(selected_lane(glider.progress) as usize))
        || enemy_shots.iter().any(|(_, shot)| {
            shot.lane == selected_lane(glider.progress) as usize && shot.depth <= 2 * LANE_UNIT
        })
    {
        game.lives -= 1;
        for (entity, _) in &enemies {
            commands.entity(entity).despawn();
        }
        for entity in &bullets {
            commands.entity(entity).despawn();
        }
        for (entity, _) in &enemy_shots {
            commands.entity(entity).despawn();
        }
        glider.progress = START_PROGRESS;
        glider.velocity = 0;
        weapon.cooldown = 0;
        **transform = glider_transform(START_PROGRESS);
        if game.lives == 0 {
            game.status = Status::GameOver;
        } else {
            reset_wave(&mut game);
            game.invulnerable = 120;
        }
        return;
    }
    if game.remaining == 0 && enemies.is_empty() && enemy_shots.is_empty() {
        game.status = Status::Clear;
    }
}

fn restart_game(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<Game>,
    mut player: Single<(&mut Glider, &mut Weapon, &mut Transform, &mut Visibility)>,
    enemies: Query<Entity, With<Flipper>>,
    bullets: Query<Entity, With<Bullet>>,
    enemy_shots: Query<Entity, With<EnemyShot>>,
) {
    if !keys.just_pressed(KeyCode::KeyR) {
        return;
    }
    for entity in enemies
        .iter()
        .chain(bullets.iter())
        .chain(enemy_shots.iter())
    {
        commands.entity(entity).despawn();
    }
    *game = Game::default();
    let (glider, weapon, transform, visibility) = &mut *player;
    glider.progress = START_PROGRESS;
    glider.velocity = 0;
    weapon.cooldown = 0;
    **transform = glider_transform(START_PROGRESS);
    **visibility = Visibility::Visible;
}

fn update_hud(game: Res<Game>, mut hud: Single<&mut Text, With<GameHud>>) {
    if !game.is_changed() {
        return;
    }
    let status = match game.status {
        Status::Playing => "",
        Status::Clear => "\nLEVEL CLEAR   R to replay",
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

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Game>()
            .init_resource::<FlipperAssets>()
            .init_resource::<explosion::BurstAssets>()
            .init_resource::<ButtonInput<KeyCode>>();
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

    fn enemy(app: &mut App, lane: usize, depth: i32, motion: Motion) -> Entity {
        let flipper = Flipper {
            lane,
            depth,
            motion,
            pause: 0,
        };
        app.world_mut()
            .spawn((
                flipper_transform(&flipper),
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
    fn rim_flips_take_28_ticks_seek_the_player_and_wrap() {
        let mut app = app();
        app.add_systems(FixedUpdate, move_enemies);
        let mut flipper = Flipper {
            lane: 15,
            depth: 0,
            motion: Motion::Pause(0),
            pause: 0,
        };
        start_flip(&mut flipper, 0);
        assert_eq!(flipper.lane, 0);
        assert_eq!(flipper.hit_lane(), None);
        assert_eq!(flipper.danger_lane(), Some(15));
        let id = app
            .world_mut()
            .spawn((
                flipper_transform(&flipper),
                flipper,
                Mesh3d::default(),
                MeshMaterial3d::<StandardMaterial>::default(),
            ))
            .id();
        ticks(&mut app, 15);
        assert_eq!(app.world().get::<Flipper>(id).unwrap().hit_lane(), Some(0));
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
        start_flip(&mut reverse, 15);
        assert_eq!(reverse.lane, 15);
        assert!(matches!(reverse.motion, Motion::Flip { direction: -1, .. }));
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
            .spawn((
                Bullet {
                    lane: 3,
                    previous_z: -9.0,
                },
                Transform::from_xyz(0.0, 0.0, -11.0),
            ))
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
            app.world_mut().spawn((
                Bullet {
                    lane: 2,
                    previous_z: -9.5,
                },
                Transform::from_xyz(0.0, 0.0, -10.0),
            ));
        }
        ticks(&mut app, 1);
        assert_eq!(app.world().resource::<Game>().score, 150);
        assert_eq!(
            app.world_mut().query::<&Bullet>().iter(app.world()).count(),
            1
        );
    }

    #[test]
    fn rim_contact_costs_a_life_resets_the_wave_and_grants_a_grace_period() {
        let mut app = app();
        app.add_systems(FixedUpdate, check_game);
        app.world_mut().resource_mut::<Game>().score = 300;
        enemy(&mut app, 7, 0, Motion::Pause(4));
        ticks(&mut app, 1);
        let game = app.world().resource::<Game>();
        assert_eq!(game.lives, 2);
        assert_eq!(game.score, 300);
        assert_eq!(game.invulnerable, 120);
        enemy(&mut app, 7, 0, Motion::Pause(4));
        ticks(&mut app, 120);
        assert_eq!(app.world().resource::<Game>().lives, 2);
        ticks(&mut app, 1);
        assert_eq!(app.world().resource::<Game>().lives, 1);
        app.world_mut().resource_mut::<Game>().invulnerable = 0;
        enemy(&mut app, 7, 0, Motion::Pause(4));
        ticks(&mut app, 1);
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
        ticks(&mut app, 1);
        assert!(app.world().resource::<Game>().status == Status::Clear);
    }

    #[test]
    fn restart_clears_enemies_shots_score_and_game_over() {
        let mut app = app();
        app.add_systems(FixedUpdate, restart_game);
        app.world_mut().resource_mut::<Game>().status = Status::GameOver;
        app.world_mut().resource_mut::<Game>().score = 150;
        enemy(&mut app, 3, REAR, Motion::Climb);
        app.world_mut().spawn(Bullet {
            lane: 2,
            previous_z: 0.0,
        });
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        ticks(&mut app, 1);
        assert_eq!(app.world().resource::<Game>().score, 0);
        assert_eq!(app.world().resource::<Game>().lives, 3);
        assert!(app.world().resource::<Game>().status == Status::Playing);
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
        let mesh = flipper_mesh();
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
        app.world_mut().spawn((
            Bullet {
                lane: 1,
                previous_z: -9.5,
            },
            Transform::from_xyz(0., 0., -10.),
        ));
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
        assert_eq!(app.world().resource::<Game>().lives, 2);
        assert!(
            app.world_mut()
                .query::<&EnemyShot>()
                .iter(app.world())
                .next()
                .is_none()
        );
    }
}
