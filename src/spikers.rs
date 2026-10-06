use super::*;

#[derive(Clone, Copy, Debug)]
enum Motion {
    Seek,
    Climb,
    Descend,
}

#[derive(Component)]
#[require(LevelActor, Enemy)]
pub(crate) struct Spiker {
    pub(crate) lane: usize,
    pub(crate) depth: i32,
    motion: Motion,
    spike: Option<Entity>,
    build: u16,
    angle: u8,
}

impl Spiker {
    pub(super) fn firing(&self) -> bool {
        !matches!(self.motion, Motion::Seek)
    }
}

#[derive(Component)]
#[require(LevelActor)]
pub(super) struct Spike {
    pub(super) lane: usize,
    pub(super) length: i32,
}

#[derive(Resource, Default)]
pub(super) struct SpikerAssets {
    body: Handle<Mesh>,
    line: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

pub(super) fn setup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let points = [
        (10., 9.),
        (9., 10.),
        (8., 9.),
        (9., 7.),
        (12., 8.),
        (10., 11.),
        (8., 11.),
        (6., 8.),
        (9., 5.),
        (14., 7.),
        (13., 14.),
        (6., 15.),
    ]
    .map(|(x, y)| Vec3::new((x - 9.) * 0.075, (9. - y) * 0.075, 0.));
    let triangles = points.windows(2).flat_map(|edge| {
        let (a, b) = (edge[0], edge[1]);
        let side = (b - a).normalize().cross(Vec3::Z) * 0.012;
        let q = [a - side, a + side, b + side, b - side].map(|p| (p, Color::WHITE));
        [[q[0], q[1], q[2]], [q[0], q[2], q[3]]]
    });
    commands.insert_resource(SpikerAssets {
        body: meshes.add(colored_mesh(triangles)),
        line: meshes.add(Cuboid::new(1., 1., 1.)),
        material: materials.add(StandardMaterial {
            base_color: cry::color(0x8f, 255),
            unlit: true,
            cull_mode: None,
            ..default()
        }),
    });
}

pub(super) fn spawn(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    assets: Res<SpikerAssets>,
    flippers: Query<&Flipper>,
    tankers: Query<&tankers::Tanker>,
    spikers: Query<&Spiker>,
) {
    let Some(generator) = &level.spikers else {
        return;
    };
    if game.status != Status::Playing || game.remaining == 0 {
        return;
    }
    if game.spiker_timer > 0 {
        game.spiker_timer -= 1;
        return;
    }
    if flippers.iter().count() + tankers.iter().count() * 3 + spikers.iter().count()
        >= level.flippers.max_active
    {
        return;
    }
    game.spiker_timer = generator.spawn_interval_ticks - 1;
    game.remaining -= 1;
    // max_spikers starts at zero and permits one before underflow.
    if !spikers.is_empty() {
        return;
    }
    game.random = game.random.wrapping_mul(1664525).wrapping_add(1013904223);
    let lane = (game.random >> 16) as usize % level.web.lanes();
    let mut pose = level.web.glider_transform(lane as i32 * LANE_UNIT);
    pose.translation.z = -level.web.world_depth() + 0.06;
    commands.spawn((
        Spiker {
            lane,
            depth: level.web.rear(),
            motion: Motion::Seek,
            spike: None,
            build: 50,
            angle: 0,
        },
        pose,
        RenderLayers::layer(1),
        Mesh3d(assets.body.clone()),
        MeshMaterial3d(assets.material.clone()),
    ));
}

#[allow(clippy::type_complexity)]
pub(super) fn advance(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    assets: Res<SpikerAssets>,
    mut spikers: Query<(&mut Spiker, &mut Transform), Without<Spike>>,
    mut spikes: Query<(Entity, &mut Spike)>,
) {
    if game.status != Status::Playing {
        return;
    }
    let Some(generator) = &level.spikers else {
        return;
    };
    for (mut spiker, mut pose) in &mut spikers {
        spiker.angle = spiker.angle.wrapping_add(4);
        match spiker.motion {
            Motion::Seek => {
                let free = (0..level.web.lanes())
                    .filter(|lane| !spikes.iter().any(|(_, s)| s.lane == *lane))
                    .collect::<Vec<_>>();
                let (entity, lane) = if free.is_empty() {
                    let (entity, spike) = spikes.iter().min_by_key(|(_, s)| s.length).unwrap();
                    (entity, spike.lane)
                } else {
                    game.random = game.random.wrapping_mul(1664525).wrapping_add(1013904223);
                    let lane = free[(game.random >> 16) as usize % free.len()];
                    let mut pose = level.web.glider_transform(lane as i32 * LANE_UNIT);
                    pose.translation.z = -level.web.world_depth();
                    let entity = commands
                        .spawn((
                            Spike {
                                lane,
                                length: 30 * LANE_UNIT,
                            },
                            pose,
                            Visibility::Visible,
                            RenderLayers::layer(1),
                            Mesh3d(assets.line.clone()),
                            MeshMaterial3d(assets.material.clone()),
                        ))
                        .id();
                    (entity, lane)
                };
                spiker.lane = lane;
                spiker.spike = Some(entity);
                spiker.build = 50;
                spiker.motion = Motion::Climb;
            }
            Motion::Climb => {
                spiker.depth -= generator.climb_speed;
                if spiker.depth < 10 * LANE_UNIT {
                    spiker.motion = Motion::Descend;
                } else if let Some(entity) = spiker.spike
                    && let Ok((_, mut spike)) = spikes.get_mut(entity)
                {
                    if level.web.rear() - spiker.depth > spike.length {
                        let length = spike.length + generator.climb_speed;
                        if length < 150 * LANE_UNIT {
                            spike.length = length;
                        }
                        if spiker.build == 0 {
                            spiker.motion = Motion::Descend;
                        } else {
                            spiker.build -= 1;
                        }
                    }
                } else {
                    spiker.motion = Motion::Descend;
                }
            }
            Motion::Descend => {
                spiker.depth += generator.climb_speed;
                if spiker.depth >= level.web.rear() {
                    spiker.depth = level.web.rear();
                    spiker.motion = Motion::Seek;
                }
            }
        }
        *pose = level.web.glider_transform(spiker.lane as i32 * LANE_UNIT);
        pose.translation.z = -(spiker.depth as f32 / LANE_UNIT as f32) * WORLD_UNIT + 0.06;
        pose.rotation = Quat::from_rotation_z(f32::from(spiker.angle) * TAU / 256.);
    }
}

pub(super) fn draw(
    level: Res<Level>,
    mut spikes: Query<(&Spike, &mut Transform, &mut Visibility)>,
) {
    for (spike, mut pose, mut visible) in &mut spikes {
        *visible = if spike.length > 0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if spike.length <= 0 {
            continue;
        }
        let mut start = level
            .web
            .glider_transform(spike.lane as i32 * LANE_UNIT)
            .translation;
        start.z = -level.web.world_depth();
        *pose = segment_transform(
            start,
            start + Vec3::Z * (spike.length as f32 / LANE_UNIT as f32) * WORLD_UNIT,
            0.018,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> App {
        let mut app = super::super::tests::app();
        app.insert_resource(Level::parse(include_str!("../levels/04.toml"), "level4").unwrap())
            .init_resource::<SpikerAssets>();
        app
    }
    fn ticks(app: &mut App, count: usize) {
        for _ in 0..count {
            app.world_mut().run_schedule(FixedUpdate);
        }
    }

    #[test]
    fn spiker_builds_fifty_one_extensions_then_returns_to_the_rear() {
        let mut app = app();
        app.add_systems(FixedUpdate, advance);
        let entity = app
            .world_mut()
            .spawn((
                Spiker {
                    lane: 7,
                    depth: 160 * LANE_UNIT,
                    motion: Motion::Seek,
                    spike: None,
                    build: 50,
                    angle: 0,
                },
                Transform::default(),
            ))
            .id();
        ticks(&mut app, 1);
        assert_eq!(
            app.world_mut()
                .query::<&Spike>()
                .single(app.world())
                .unwrap()
                .length,
            30 * LANE_UNIT
        );
        ticks(&mut app, 87);
        assert!(matches!(
            app.world().get::<Spiker>(entity).unwrap().motion,
            Motion::Descend
        ));
        assert_eq!(
            app.world_mut()
                .query::<&Spike>()
                .single(app.world())
                .unwrap()
                .length,
            30 * LANE_UNIT + 51 * 0xd000
        );
        assert_eq!(
            app.world().get::<Spiker>(entity).unwrap().depth,
            (160 * LANE_UNIT) - 87 * 0xd000
        );
        ticks(&mut app, 87);
        assert!(
            matches!(
                app.world().get::<Spiker>(entity).unwrap().motion,
                Motion::Seek
            ),
            "depth={}, motion={:?}",
            app.world().get::<Spiker>(entity).unwrap().depth,
            app.world().get::<Spiker>(entity).unwrap().motion
        );
        assert_eq!(
            app.world().get::<Spiker>(entity).unwrap().depth,
            160 * LANE_UNIT
        );
    }

    #[test]
    fn shots_trim_spikes_by_three_units_and_leave_particle_shots_alive() {
        for particle in [false, true] {
            let mut app = app();
            app.add_systems(FixedUpdate, resolve_hits);
            let spike = app
                .world_mut()
                .spawn(Spike {
                    lane: 7,
                    length: 30 * LANE_UNIT,
                })
                .id();
            for _ in 0..11 {
                let length = app.world().get::<Spike>(spike).unwrap().length;
                let shot = app
                    .world_mut()
                    .spawn((
                        Bullet { lane: 7 },
                        Transform::from_xyz(
                            0.,
                            0.,
                            0.06 - (160. - length as f32 / LANE_UNIT as f32) * WORLD_UNIT,
                        ),
                    ))
                    .id();
                if particle {
                    app.world_mut()
                        .entity_mut(shot)
                        .insert(ParticleShot::default());
                }
                ticks(&mut app, 1);
                assert_eq!(app.world().get_entity(shot).is_ok(), particle);
                if particle {
                    app.world_mut().despawn(shot);
                }
            }
            assert!(app.world().get_entity(spike).is_err());
            assert_eq!(app.world().resource::<Game>().score, 36);
            assert_eq!(
                app.world_mut()
                    .query::<&powerups::PowerUp>()
                    .iter(app.world())
                    .count(),
                0
            );
        }
    }

    #[test]
    fn spikes_do_not_hold_the_wave_open_but_can_kill_the_claw_during_exit() {
        let mut app = app();
        app.add_systems(FixedUpdate, (exit::animate, check_game).chain());
        app.world_mut().resource_mut::<Game>().remaining = 0;
        app.world_mut().spawn(Spike {
            lane: 7,
            length: 150 * LANE_UNIT,
        });
        ticks(&mut app, 1);
        assert!(app.world().resource::<Game>().status == Status::Exiting);
        ticks(&mut app, 36);
        assert!(app.world().resource::<Game>().status == Status::Spiked);
        assert_eq!(app.world().resource::<Game>().lives, 3);
        ticks(&mut app, 100);
        assert_eq!(app.world().resource::<Game>().lives, 2);
        assert!(app.world().resource::<Game>().status == Status::Entering);
    }

    #[test]
    fn a_living_spiker_blocks_completion_and_only_one_is_generated_at_a_time() {
        let mut app = app();
        let level = app.world().resource::<Level>().clone();
        reset_wave(&mut app.world_mut().resource_mut::<Game>(), &level);
        app.world_mut().resource_mut::<Game>().status = Status::Playing;
        app.world_mut().resource_mut::<Game>().entry = None;
        app.add_systems(FixedUpdate, spawn);
        ticks(&mut app, 400);
        assert_eq!(app.world().resource::<Game>().remaining, 25);
        ticks(&mut app, 1);
        assert_eq!(app.world().resource::<Game>().remaining, 24);
        ticks(&mut app, 801);
        assert_eq!(app.world().resource::<Game>().remaining, 23);
        assert_eq!(
            app.world_mut().query::<&Spiker>().iter(app.world()).count(),
            1
        );
        app.world_mut().resource_mut::<Game>().remaining = 0;
        app.add_systems(FixedUpdate, check_game);
        ticks(&mut app, 1);
        assert!(app.world().resource::<Game>().status == Status::Playing);
    }
}
