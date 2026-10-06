use super::*;

#[derive(Component)]
pub(super) struct Beast {
    pub(super) stage: usize,
}

#[derive(Component)]
#[require(ShotSlot)]
pub(super) struct Reflected {
    pub(super) speed: i32,
}

#[derive(Resource, Default)]
pub(super) struct BeastAssets {
    pub(super) frames: [Handle<Mesh>; 3],
    pub(super) parts: [Handle<Mesh>; 2],
}

fn part(
    points: &[(f32, f32)],
    faces: &[(u8, [usize; 3], [u8; 3])],
    z: f32,
) -> Vec<[(Vec3, Color); 3]> {
    faces
        .iter()
        .map(|(hue, face, intensity)| {
            std::array::from_fn(|i| {
                let (x, y) = points[face[i]];
                (
                    Vec3::new((x - 9.) * 0.075, (9. - y) * 0.075, z),
                    cry::color(*hue, intensity[i]),
                )
            })
        })
        .collect()
}

pub(super) fn setup(commands: &mut Commands, meshes: &mut Assets<Mesh>) {
    let body = part(
        &[
            (3., 7.),
            (7., 5.),
            (11., 5.),
            (15., 7.),
            (15., 11.),
            (11., 13.),
            (7., 13.),
            (3., 11.),
            (5., 7.),
            (7., 7.),
            (7., 9.),
            (13., 7.),
            (11., 7.),
            (11., 9.),
        ],
        &[
            (0x88, [0, 1, 2], [128, 64, 64]),
            (0x88, [0, 2, 3], [128, 64, 128]),
            (0x88, [0, 7, 4], [128, 96, 96]),
            (0x88, [0, 3, 4], [128, 128, 96]),
            (0x88, [7, 6, 4], [96, 64, 96]),
            (0x88, [6, 5, 4], [64, 64, 96]),
            (0xf0, [8, 9, 10], [192, 240, 128]),
            (0xf0, [11, 12, 13], [192, 240, 128]),
        ],
        0.,
    );
    let faces = [
        (0xbb, [0, 1, 2], [240, 240, 192]),
        (0xbc, [1, 2, 3], [240, 192, 160]),
        (0xcc, [2, 3, 4], [192, 160, 160]),
        (0x66, [5, 6, 7], [160, 192, 255]),
    ];
    let left = part(
        &[
            (-7., -11.),
            (-7., 1.),
            (-3., -3.),
            (-3., 5.),
            (5., 1.),
            (17., 13.),
            (13., 13.),
            (13., 25.),
        ],
        &faces,
        0.01,
    );
    let right = part(
        &[
            (25., -11.),
            (25., 1.),
            (21., -3.),
            (21., 5.),
            (13., 1.),
            (1., 13.),
            (5., 13.),
            (5., 25.),
        ],
        &faces,
        0.02,
    );
    let frames = [
        meshes.add(colored_mesh(body.clone())),
        meshes.add(colored_mesh(body.iter().chain(&left).copied())),
        meshes.add(colored_mesh(
            body.into_iter().chain(left.clone()).chain(right.clone()),
        )),
    ];
    commands.insert_resource(BeastAssets {
        frames,
        parts: [
            meshes.add(colored_mesh(left)),
            meshes.add(colored_mesh(right)),
        ],
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn(
    level: Res<Level>,
    mut commands: Commands,
    mut game: ResMut<Game>,
    assets: Res<FlipperAssets>,
    enemies: Query<&Flipper>,
    tankers: Query<&tankers::Tanker>,
    spikers: Query<&spikers::Spiker>,
) {
    let Some(generator) = &level.beasts else {
        return;
    };
    if game.status != Status::Playing || game.remaining == 0 {
        return;
    }
    if game.beast_timer > 0 {
        game.beast_timer -= 1;
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
        pause: 0,
    };
    commands.spawn((
        flipper_transform(&flipper, &level.web),
        flipper,
        Beast { stage: 2 },
        RenderLayers::layer(1),
        Mesh3d(assets.point.clone()),
        MeshMaterial3d(assets.point_shades.first().cloned().unwrap_or_default()),
    ));
    game.remaining -= 1;
    game.beast_timer = generator.spawn_interval_ticks - 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn three_rail_hits_strip_two_horns_then_kill_the_core() {
        let mut app = super::super::tests::app();
        app.insert_resource(Level::parse(include_str!("../levels/10.toml"), "level10").unwrap())
            .add_systems(FixedUpdate, resolve_hits);
        let beast = app
            .world_mut()
            .spawn((
                Beast { stage: 2 },
                Flipper {
                    lane: 5,
                    depth: 60 * LANE_UNIT,
                    motion: Motion::Climb,
                    pause: 0,
                },
                Transform::default(),
                Mesh3d::default(),
            ))
            .id();
        for stage in [1, 0] {
            app.world_mut()
                .spawn((Bullet { lane: 5 }, Transform::from_xyz(0., 0., -10.)));
            app.world_mut().run_schedule(FixedUpdate);
            assert_eq!(app.world().get::<Beast>(beast).unwrap().stage, stage);
            assert_eq!(app.world().resource::<Game>().score, 0);
        }
        assert_eq!(
            app.world_mut()
                .query::<&Reflected>()
                .iter(app.world())
                .count(),
            2
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<ShotSlot>>()
                .iter(app.world())
                .count(),
            2
        );
        app.world_mut()
            .spawn((Bullet { lane: 5 }, Transform::from_xyz(0., 0., -10.)));
        app.world_mut().run_schedule(FixedUpdate);
        assert!(app.world().get_entity(beast).is_err());
        assert_eq!(app.world().resource::<Game>().score, 150);
    }
    #[test]
    fn reflected_horns_absorb_normal_shots_without_disappearing() {
        let mut app = super::super::tests::app();
        app.add_systems(FixedUpdate, resolve_hits);
        let fragment = app
            .world_mut()
            .spawn((
                EnemyShot {
                    lane: 3,
                    depth: 60 * LANE_UNIT,
                    angle: 0,
                },
                Reflected { speed: 0x1c000 },
                Transform::default(),
            ))
            .id();
        let shot = app
            .world_mut()
            .spawn((Bullet { lane: 3 }, Transform::from_xyz(0., 0., -10.)))
            .id();
        app.world_mut().run_schedule(FixedUpdate);
        assert!(app.world().get_entity(fragment).is_ok());
        assert!(app.world().get_entity(shot).is_err());
    }
}
