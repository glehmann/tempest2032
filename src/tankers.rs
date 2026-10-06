use super::*;

#[derive(Component)]
#[require(LevelActor, Enemy)]
pub(crate) struct Tanker {
    pub(crate) lane: usize,
    pub(crate) depth: i32,
    pub(crate) approach: bool,
}

#[derive(Component)]
pub(super) struct Released {
    pub(super) burst: bool,
    pub(super) fast: bool,
    pub(super) remaining: u8,
    pub(super) direction: i32,
}

#[derive(Resource, Default)]
pub(super) struct TankerAssets {
    body: Handle<Mesh>,
    pub(super) fast: Handle<Mesh>,
    point: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    shades: Vec<Handle<StandardMaterial>>,
}

pub(super) fn setup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let vertices = [
        (9., 1.),
        (17., 9.),
        (9., 17.),
        (1., 9.),
        (9., 4.),
        (14., 9.),
        (9., 14.),
        (4., 9.),
        (6., 7.),
        (12., 7.),
        (6., 11.),
        (12., 11.),
        (7., 9.),
        (11., 9.),
        (9., 9.),
    ]
    .map(|(x, y)| Vec3::new((x - 9.) * 0.075, (9. - y) * 0.075, 0.0));
    let mut triangles = Vec::new();
    for (hue, face) in [
        (0x92, [3, 0, 7]),
        (0x93, [7, 4, 0]),
        (0x92, [0, 1, 4]),
        (0x93, [4, 5, 1]),
        (0x92, [1, 2, 5]),
        (0x93, [5, 6, 2]),
        (0x92, [2, 3, 6]),
        (0x93, [6, 7, 3]),
        (0xf0, [8, 12, 14]),
        (0x30, [9, 13, 14]),
        (0x30, [10, 12, 14]),
        (0xf0, [11, 13, 14]),
    ] {
        let intensity = if face[2] == 14 {
            [128, 192, 255]
        } else {
            [240, 128, 64]
        };
        triangles.push(std::array::from_fn(|i| {
            (vertices[face[i]], cry::color(hue, intensity[i]))
        }));
    }
    let fast_vertices = [
        (1., 9.),
        (5., 9.),
        (9., 5.),
        (9., 13.),
        (13., 9.),
        (17., 9.),
    ]
    .map(|(x, y)| Vec3::new((x - 9.) * 0.075, (9. - y) * 0.075, 0.0));
    let fast = colored_mesh(
        [
            (0x44, [0, 1, 2]),
            (0x66, [5, 4, 2]),
            (0x44, [0, 1, 3]),
            (0x66, [5, 4, 3]),
        ]
        .map(|(hue, face)| {
            std::array::from_fn(|i| (fast_vertices[face[i]], cry::color(hue, [255, 128, 192][i])))
        }),
    );
    commands.insert_resource(TankerAssets {
        body: meshes.add(colored_mesh(triangles)),
        fast: meshes.add(fast),
        point: meshes.add(Rectangle::new(1., 1.)),
        material: materials.add(StandardMaterial {
            unlit: true,
            cull_mode: None,
            ..default()
        }),
        shades: (0..=255)
            .map(|i| {
                materials.add(StandardMaterial {
                    base_color: cry::color(0x80, i),
                    unlit: true,
                    cull_mode: None,
                    ..default()
                })
            })
            .collect(),
    });
}

pub(super) fn transform(tanker: &Tanker, web: &level::Web) -> Transform {
    let mut transform = web.glider_transform(tanker.lane as i32 * LANE_UNIT);
    transform.translation.z = -(tanker.depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
    transform.rotation = web.orientation(tanker.lane);
    transform
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    assets: Res<TankerAssets>,
    flippers: Query<&Flipper>,
    tankers: Query<&Tanker>,
    spikers: Query<&spikers::Spiker>,
    fuses: Query<&fuseballs::Fuseball>,
) {
    let Some(generator) = &level.tankers else {
        return;
    };
    if game.status != Status::Playing || game.remaining == 0 {
        return;
    }
    if game.tanker_timer > 0 {
        game.tanker_timer -= 1;
        return;
    }
    if flippers.iter().count()
        + tankers.iter().count() * 3
        + spikers.iter().count()
        + fuses.iter().count()
        >= level.flippers.max_active
    {
        return;
    }
    game.random = game.random.wrapping_mul(1664525).wrapping_add(1013904223);
    let tanker = Tanker {
        lane: 1 + (game.random >> 16) as usize % (level.web.lanes() - 2),
        depth: level.web.rear() + 300 * LANE_UNIT,
        approach: true,
    };
    commands.spawn((
        transform(&tanker, &level.web),
        tanker,
        RenderLayers::layer(1),
        Mesh3d(assets.point.clone()),
        MeshMaterial3d(assets.shades.first().cloned().unwrap_or_default()),
    ));
    game.remaining -= 1;
    game.tanker_timer = generator.spawn_interval_ticks - 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_generators_share_the_original_seventeen_spawn_budget() {
        let mut app = super::super::tests::app();
        let level = Level::parse(include_str!("../levels/03.toml"), "level3").unwrap();
        reset_wave(&mut app.world_mut().resource_mut::<Game>(), &level);
        app.world_mut().resource_mut::<Game>().status = Status::Playing;
        app.world_mut().resource_mut::<Game>().entry = None;
        app.insert_resource(level)
            .add_systems(FixedUpdate, (spawn_enemies, spawn).chain());
        for _ in 0..222 {
            app.world_mut().run_schedule(FixedUpdate);
        }
        assert_eq!(
            app.world_mut().query::<&Tanker>().iter(app.world()).count(),
            0
        );
        assert_eq!(app.world().resource::<Game>().remaining, 15);
        app.world_mut().run_schedule(FixedUpdate);
        let tanker = app
            .world_mut()
            .query::<&Tanker>()
            .single(app.world())
            .unwrap();
        assert!((1..10).contains(&tanker.lane));
        assert_eq!(app.world().resource::<Game>().remaining, 14);
    }

    #[test]
    fn shooting_a_tanker_releases_two_children_without_advancing_the_drop_counter() {
        let mut app = super::super::tests::app();
        let mut level = Level::parse(include_str!("../levels/03.toml"), "level3").unwrap();
        level.tankers.as_mut().unwrap().super_chance = 0;
        app.insert_resource(level)
            .add_systems(FixedUpdate, resolve_hits);
        app.world_mut().spawn((
            Tanker {
                lane: 5,
                depth: 60 * LANE_UNIT,
                approach: false,
            },
            Transform::default(),
        ));
        app.world_mut()
            .spawn((Bullet { lane: 5 }, Transform::from_xyz(0., 0., -10.)));
        app.world_mut().run_schedule(FixedUpdate);
        assert_eq!(app.world().resource::<Game>().score, 100);
        assert_eq!(
            app.world_mut().query::<&Tanker>().iter(app.world()).count(),
            0
        );
        assert_eq!(
            app.world_mut()
                .query::<&powerups::PowerUp>()
                .iter(app.world())
                .count(),
            0
        );
        let mut lanes = app
            .world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .map(|f| f.lane)
            .collect::<Vec<_>>();
        lanes.sort();
        assert_eq!(lanes, [4, 6]);
        app.add_systems(FixedUpdate, move_enemies);
        for _ in 0..34 {
            app.world_mut().run_schedule(FixedUpdate);
        }
        assert!(
            app.world_mut()
                .query::<&Flipper>()
                .iter(app.world())
                .all(|f| matches!(f.motion, Motion::Climb) && f.depth < 60 * LANE_UNIT)
        );
    }
}

pub(super) fn open(
    commands: &mut Commands,
    game: &mut Game,
    level: &Level,
    assets: &FlipperAssets,
    shells: &TankerAssets,
    tanker: &Tanker,
) {
    game.score += 100;
    for direction in [-1, 1] {
        game.random = game.random.wrapping_mul(1664525).wrapping_add(1013904223);
        let fast =
            (game.random >> 24) < u32::from(level.tankers.as_ref().map_or(0, |g| g.super_chance));
        let lane = level.web.neighbor(tanker.lane, direction).unwrap();
        let flipper = Flipper {
            lane,
            depth: tanker.depth,
            motion: Motion::Flip {
                from: tanker.lane,
                direction,
                angle: 0,
            },
            pause: if fast { 0 } else { level.flippers.pause_ticks },
        };
        let remaining = if fast {
            ((game.random >> 16) & 3) as u8 + 1
        } else {
            0
        };
        commands.spawn((
            flipper_transform(&flipper, &level.web),
            flipper,
            Released {
                burst: true,
                fast,
                remaining,
                direction,
            },
            RenderLayers::layer(1),
            Mesh3d(if fast {
                shells.fast.clone()
            } else {
                assets.body.clone()
            }),
            MeshMaterial3d(assets.shaded.clone()),
        ));
    }
}

#[allow(clippy::type_complexity)]
pub(super) fn move_tankers(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    assets: Res<FlipperAssets>,
    shells: Res<TankerAssets>,
    bursts: Res<explosion::BurstAssets>,
    mut tankers: Query<(
        Entity,
        &mut Tanker,
        &mut Transform,
        &mut Mesh3d,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    if game.status != Status::Playing {
        return;
    }
    let Some(generator) = &level.tankers else {
        return;
    };
    for (entity, mut tanker, mut pose, mut mesh, mut material) in &mut tankers {
        if tanker.approach {
            tanker.depth -= 2 * LANE_UNIT;
            if tanker.depth <= level.web.rear() {
                tanker.depth = level.web.rear();
                tanker.approach = false;
                mesh.0 = shells.body.clone();
                material.0 = shells.material.clone();
            }
        } else {
            tanker.depth = (tanker.depth - generator.climb_speed).max(0);
        }
        *pose = transform(&tanker, &level.web);
        if !tanker.approach && tanker.depth == 0 {
            open(&mut commands, &mut game, &level, &assets, &shells, &tanker);
            explosion::spawn_explosion(&mut commands, &bursts, pose.translation);
            commands.entity(entity).despawn();
        }
    }
}

#[allow(clippy::type_complexity)]
pub(super) fn draw_points(
    assets: Res<TankerAssets>,
    cameras: Query<(&Transform, &Projection), (With<Camera3d>, Without<Tanker>)>,
    mut tankers: Query<(
        &Tanker,
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let Some((camera, Projection::Perspective(projection))) = cameras.iter().next() else {
        return;
    };
    for (tanker, mut transform, mut material) in &mut tankers {
        if tanker.approach {
            let (width, intensity) = approach_dot_style(transform.translation, camera, projection);
            transform.rotation = camera.rotation;
            transform.scale = Vec3::splat(width);
            material.0 = assets.shades[intensity].clone();
        }
    }
}
