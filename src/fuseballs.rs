use super::*;

#[derive(Clone, Copy)]
enum Motion {
    Approach,
    Rail,
    Cross {
        from: usize,
        to: usize,
        lane: usize,
        steps: u8,
        delay: u8,
    },
}

#[derive(Component)]
#[require(LevelActor, Enemy)]
pub(crate) struct Fuseball {
    pub(crate) depth: i32,
    pub(crate) edge: usize,
    motion: Motion,
    rise: u8,
    tick: u16,
}

impl Fuseball {
    pub(crate) fn target(&self) -> Option<(usize, i32)> {
        match self.motion {
            Motion::Cross {
                lane,
                steps: 3..=11,
                ..
            } => Some((lane, self.depth)),
            _ => None,
        }
    }
    fn danger(&self) -> Option<usize> {
        if self.depth <= 0 {
            match self.motion {
                Motion::Cross { lane, .. } => Some(lane),
                _ => None,
            }
        } else {
            None
        }
    }
}

#[derive(Resource, Default)]
pub(super) struct FuseAssets {
    frames: Vec<Handle<Mesh>>,
    point: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    shades: Vec<Handle<StandardMaterial>>,
}

pub(super) fn setup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let mut frames = Vec::new();
    for pattern in 0..32 {
        let mut triangles = Vec::new();
        for (arm, hue) in [15, 255, 128, 143, 240].into_iter().enumerate() {
            let points = if pattern & (1 << arm) == 0 {
                [(1., 9.), (5., 6.), (5., 8.), (9., 9.)]
            } else {
                [(1., 9.), (5., 12.), (5., 10.), (9., 9.)]
            };
            let rotation = Quat::from_rotation_z(-(arm as f32) * 51. * TAU / 256.);
            let points =
                points.map(|(x, y)| rotation * Vec3::new((x - 9.) * 0.075, (9. - y) * 0.075, 0.));
            for (face, intensity) in [([0, 1, 2], [64, 128, 96]), ([3, 1, 2], [255, 128, 96])] {
                triangles.push(std::array::from_fn(|i| {
                    (points[face[i]], cry::color(hue, intensity[i]))
                }));
            }
        }
        frames.push(meshes.add(colored_mesh(triangles)));
    }
    commands.insert_resource(FuseAssets {
        frames,
        point: meshes.add(Rectangle::new(1., 1.)),
        material: materials.add(StandardMaterial {
            unlit: true,
            cull_mode: None,
            ..default()
        }),
        shades: (0..=255)
            .map(|i| {
                materials.add(StandardMaterial {
                    base_color: cry::color(15, i),
                    unlit: true,
                    cull_mode: None,
                    ..default()
                })
            })
            .collect(),
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    assets: Res<FuseAssets>,
    flippers: Query<&Flipper>,
    tankers: Query<&tankers::Tanker>,
    spikers: Query<&spikers::Spiker>,
    fuses: Query<&Fuseball>,
) {
    let Some(g) = &level.fuseballs else {
        return;
    };
    if game.status != Status::Playing || game.remaining == 0 {
        return;
    }
    if game.fuse_timer > 0 {
        game.fuse_timer -= 1;
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
    let lane = (game.random >> 16) as usize % level.web.lanes();
    let edge = if level.web.closed {
        (lane + 1) % level.web.lanes()
    } else {
        lane + 1
    };
    let fuse = Fuseball {
        edge,
        depth: level.web.rear() + 300 * LANE_UNIT,
        motion: Motion::Approach,
        rise: 48,
        tick: 0,
    };
    commands.spawn((
        fuse,
        Transform::from_translation(level.web.vertex(
            edge as f32,
            FRONT_RADIUS,
            -(level.web.depth as f32 + 300.) * WORLD_UNIT,
        )),
        RenderLayers::layer(1),
        Mesh3d(assets.point.clone()),
        MeshMaterial3d(assets.shades.first().cloned().unwrap_or_default()),
    ));
    game.remaining -= 1;
    game.fuse_timer = g.spawn_interval_ticks - 1;
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn advance(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut upgrades: ResMut<powerups::Upgrades>,
    mut player: Single<(&mut Glider, &mut Weapon, &Transform, &mut Visibility)>,
    assets: Res<FuseAssets>,
    bursts: Res<explosion::BurstAssets>,
    bullets: Query<(Entity, &Bullet, &Transform, Option<&ParticleShot>)>,
    mut fuses: Query<
        (
            Entity,
            &mut Fuseball,
            &mut Transform,
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        (Without<Glider>, Without<Bullet>),
    >,
) {
    if game.status != Status::Playing {
        return;
    }
    let Some(g) = &level.fuseballs else {
        return;
    };
    let (glider, weapon, player_pose, visibility) = &mut *player;
    let player_lane = level.web.selected_lane(glider.progress) as usize;
    let mut consumed = HashSet::new();
    for (entity, mut fuse, mut pose, mut mesh, mut material) in &mut fuses {
        if let Some((lane, depth)) = fuse.target()
            && let Some((shot, _, _, particle)) = bullets.iter().find(|(shot, b, p, _)| {
                !consumed.contains(shot)
                    && b.lane == lane
                    && (p.translation.z - (-depth as f32 / LANE_UNIT as f32 * WORLD_UNIT + 0.06))
                        .abs()
                        < 6. * WORLD_UNIT
            })
        {
            if particle.is_none() {
                commands.entity(shot).despawn();
                consumed.insert(shot);
            }
            game.random = game.random.wrapping_mul(1664525).wrapping_add(1013904223);
            game.score += 250 * (1 + ((game.random >> 16) % 3));
            commands.entity(entity).despawn();
            explosion::spawn_explosion(&mut commands, &bursts, pose.translation);
            continue;
        }
        if game.invulnerable == 0 && upgrades.jump_height == 0 && fuse.danger() == Some(player_lane)
        {
            game.status = Status::Fried;
            game.death = Some(shot_death::Death::new(player_pose.translation, &level.web));
            glider.velocity = 0;
            **weapon = Weapon::default();
            **visibility = Visibility::Hidden;
            upgrades.lose_life();
            return;
        }
        fuse.tick = fuse.tick.wrapping_add(1);
        match fuse.motion {
            Motion::Approach => {
                fuse.depth -= 2 * LANE_UNIT;
                if fuse.depth <= level.web.rear() {
                    fuse.depth = level.web.rear();
                    fuse.motion = Motion::Rail;
                    material.0 = assets.material.clone();
                }
            }
            Motion::Rail => {
                if fuse.depth >= 0 {
                    fuse.depth -= g.climb_speed;
                }
                if fuse.rise > 0 {
                    fuse.rise -= 1;
                } else {
                    fuse.rise = 48;
                    let reference = if level.web.closed {
                        (fuse.edge + level.web.lanes() - 1) % level.web.lanes()
                    } else {
                        fuse.edge.saturating_sub(1)
                    };
                    let delta = level
                        .web
                        .lane_delta(reference as i32 * LANE_UNIT, player_lane as i32 * LANE_UNIT);
                    let mut direction = if delta > 0 { 1 } else { -1 };
                    let edge_count = if level.web.closed {
                        level.web.lanes()
                    } else {
                        level.web.lanes() + 1
                    };
                    if !level.web.closed
                        && ((direction < 0 && fuse.edge == 0)
                            || (direction > 0 && fuse.edge == level.web.lanes()))
                    {
                        direction = -direction;
                    }
                    let to = (fuse.edge as i32 + direction).rem_euclid(edge_count as i32) as usize;
                    let lane = if direction > 0 { fuse.edge } else { to };
                    fuse.motion = Motion::Cross {
                        from: fuse.edge,
                        to,
                        lane,
                        steps: 0,
                        delay: 4,
                    };
                }
            }
            Motion::Cross {
                from,
                to,
                lane,
                steps,
                delay,
            } => {
                if delay > 0 {
                    fuse.motion = Motion::Cross {
                        from,
                        to,
                        lane,
                        steps,
                        delay: delay - 1,
                    };
                } else if steps == 15 {
                    fuse.edge = to;
                    fuse.motion = Motion::Rail;
                } else {
                    fuse.motion = Motion::Cross {
                        from,
                        to,
                        lane,
                        steps: steps + 1,
                        delay: 4,
                    };
                }
            }
        }
        let z = -fuse.depth as f32 / LANE_UNIT as f32 * WORLD_UNIT + 0.06;
        *pose = Transform::from_translation(match fuse.motion {
            Motion::Cross {
                from, to, steps, ..
            } => level.web.vertex(from as f32, FRONT_RADIUS, z).lerp(
                level.web.vertex(to as f32, FRONT_RADIUS, z),
                f32::from(steps) / 16.,
            ),
            _ => level.web.vertex(fuse.edge as f32, FRONT_RADIUS, z),
        });
        if !matches!(fuse.motion, Motion::Approach) {
            pose.rotation =
                Quat::from_rotation_z(-f32::from((fuse.tick as u8).wrapping_mul(2)) * TAU / 256.);
            let pattern = (u32::from(fuse.tick / 4)
                .wrapping_mul(1103515245)
                .wrapping_add(12345)
                >> 16)
                & 31;
            if let Some(frame) = assets.frames.get(pattern as usize) {
                mesh.0 = frame.clone();
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub(super) fn draw_points(
    assets: Res<FuseAssets>,
    cameras: Query<(&Transform, &Projection), (With<Camera3d>, Without<Fuseball>)>,
    mut fuses: Query<(
        &Fuseball,
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let Some((camera, Projection::Perspective(projection))) = cameras.iter().next() else {
        return;
    };
    for (fuse, mut pose, mut material) in &mut fuses {
        if matches!(fuse.motion, Motion::Approach) {
            let (width, intensity) = approach_dot_style(pose.translation, camera, projection);
            pose.rotation = camera.rotation;
            pose.scale = Vec3::splat(width);
            if let Some(shade) = assets.shades.get(intensity) {
                material.0 = shade.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> App {
        let mut app = super::super::tests::app();
        app.insert_resource(Level::parse(include_str!("../levels/11.toml"), "level11").unwrap())
            .init_resource::<FuseAssets>()
            .add_systems(FixedUpdate, advance);
        app
    }
    fn ticks(app: &mut App, n: usize) {
        for _ in 0..n {
            app.world_mut().run_schedule(FixedUpdate);
        }
    }
    #[test]
    fn fuseballs_only_take_hits_in_the_middle_of_a_crossing() {
        for steps in [0, 3, 8, 11, 15] {
            let mut app = app();
            let entity = app
                .world_mut()
                .spawn((
                    Fuseball {
                        depth: 60 * LANE_UNIT,
                        edge: 4,
                        motion: Motion::Cross {
                            from: 4,
                            to: 5,
                            lane: 4,
                            steps,
                            delay: 4,
                        },
                        rise: 48,
                        tick: 0,
                    },
                    Transform::default(),
                    Mesh3d::default(),
                    MeshMaterial3d::<StandardMaterial>::default(),
                ))
                .id();
            app.world_mut()
                .spawn((Bullet { lane: 4 }, Transform::from_xyz(0., 0., -10.)));
            ticks(&mut app, 1);
            assert_eq!(
                app.world().get_entity(entity).is_err(),
                (3..=11).contains(&steps)
            );
            if (3..=11).contains(&steps) {
                assert!([250, 500, 750].contains(&app.world().resource::<Game>().score));
            }
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
    fn rail_rise_and_sixteen_five_tick_steps_match_the_original_counters() {
        let mut app = app();
        let entity = app
            .world_mut()
            .spawn((
                Fuseball {
                    depth: 60 * LANE_UNIT,
                    edge: 4,
                    motion: Motion::Rail,
                    rise: 48,
                    tick: 0,
                },
                Transform::default(),
                Mesh3d::default(),
                MeshMaterial3d::<StandardMaterial>::default(),
            ))
            .id();
        ticks(&mut app, 48);
        assert!(matches!(
            app.world().get::<Fuseball>(entity).unwrap().motion,
            Motion::Rail
        ));
        ticks(&mut app, 1);
        assert!(matches!(
            app.world().get::<Fuseball>(entity).unwrap().motion,
            Motion::Cross { .. }
        ));
        assert_eq!(
            app.world().get::<Fuseball>(entity).unwrap().depth,
            60 * LANE_UNIT - 49 * 0x8000
        );
        ticks(&mut app, 79);
        assert!(matches!(
            app.world().get::<Fuseball>(entity).unwrap().motion,
            Motion::Cross { .. }
        ));
        ticks(&mut app, 1);
        assert!(matches!(
            app.world().get::<Fuseball>(entity).unwrap().motion,
            Motion::Rail
        ));
    }
    #[test]
    fn a_rim_crossing_fries_the_claw_even_while_the_weapon_is_active() {
        let mut app = app();
        app.world_mut().spawn((
            Fuseball {
                depth: 0,
                edge: 7,
                motion: Motion::Cross {
                    from: 7,
                    to: 8,
                    lane: 7,
                    steps: 0,
                    delay: 4,
                },
                rise: 48,
                tick: 0,
            },
            Transform::default(),
            Mesh3d::default(),
            MeshMaterial3d::<StandardMaterial>::default(),
        ));
        let entity = app
            .world_mut()
            .query_filtered::<Entity, With<Glider>>()
            .single(app.world())
            .unwrap();
        app.world_mut().get_mut::<Weapon>(entity).unwrap().firing = true;
        ticks(&mut app, 1);
        assert!(app.world().resource::<Game>().status == Status::Fried);
    }
}
