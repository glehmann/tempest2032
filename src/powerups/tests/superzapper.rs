use super::*;
use crate::{
    enemies::{aliens, beasts::Beast, mirrors::Mirror, spikers::Spike},
    level::LoadReason,
    test_support::{enemy, ticks},
    web::LANE_UNIT,
};

fn app() -> App {
    let mut app = crate::test_support::app();
    app.init_resource::<Time<Virtual>>()
        .add_systems(PreUpdate, queue.run_if(crate::pause::running))
        .add_systems(FixedUpdate, advance.run_if(crate::pause::running));
    app
}

fn press(app: &mut App) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.reset(KeyCode::KeyC);
    keys.press(KeyCode::KeyC);
    app.world_mut().run_schedule(PreUpdate);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
}

fn shot(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            EnemyShot {
                lane: 3,
                depth: 50 * LANE_UNIT,
                angle: 0,
            },
            Transform::default(),
        ))
        .id()
}

#[test]
fn one_press_matches_the_recorded_jaguar_three_flipper_clear() {
    let mut app = app();
    for lane in 0..3 {
        enemy(&mut app, lane, 40 * LANE_UNIT, Motion::Climb);
    }
    press(&mut app);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset(KeyCode::KeyC);
    // The recording scores 150, 300, 450 at 5.370, 5.437, 5.503 seconds.
    for (elapsed_ticks, score) in [(1, 150), (3, 150), (1, 300), (3, 300), (1, 450)] {
        ticks(&mut app, elapsed_ticks);
        assert_eq!(app.world().resource::<Game>().score, score);
        assert_eq!(
            app.world_mut()
                .query::<&Flipper>()
                .iter(app.world())
                .count(),
            3 - (score / 150) as usize,
        );
    }
    assert_eq!(app.world().resource::<Upgrades>().zapper.uses, 1);
    assert!(app.world().resource::<Upgrades>().zapper.bolt.is_some());
    assert_eq!(app.world().resource::<Upgrades>().zapper.bolt_ticks, 4);
}

#[test]
fn a_short_tap_starts_a_four_tick_sweep_without_spending_the_second_use_while_held() {
    let mut app = app();
    for lane in 0..3 {
        enemy(&mut app, lane, 40 * LANE_UNIT, Motion::Climb);
    }
    shot(&mut app);
    press(&mut app);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyC);
    ticks(&mut app, 1);
    let count = |app: &mut App| {
        app.world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .count()
            + app
                .world_mut()
                .query::<&EnemyShot>()
                .iter(app.world())
                .count()
    };
    assert_eq!(count(&mut app), 3);
    press(&mut app); // A held press during a sweep cannot queue the second use.
    ticks(&mut app, 3);
    assert_eq!(count(&mut app), 3);
    ticks(&mut app, 13);
    assert_eq!(
        app.world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(
        app.world_mut()
            .query::<&EnemyShot>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(app.world().resource::<Game>().score, 450);
    assert_eq!(app.world().resource::<Upgrades>().zapper.uses, 1);
    assert!(!app.world().resource::<Upgrades>().zapper.active);
    assert_eq!(
        app.world_mut()
            .query::<&crate::powerups::PowerUp>()
            .iter(app.world())
            .count(),
        0
    );
    assert!(app.world_mut().resource_mut::<Upgrades>().drop_due(5));
    assert_eq!(
        app.world()
            .resource::<Game>()
            .announcements
            .iter()
            .filter(|e| e.id == Id::EatElectricDeath)
            .count(),
        1
    );
    app.world_mut().run_schedule(PreUpdate);
    ticks(&mut app, 4);
    assert_eq!(app.world().resource::<Upgrades>().zapper.uses, 1);
}

#[test]
fn a_zap_and_bullet_in_the_same_tick_cannot_score_the_same_enemy_twice() {
    let mut app = app();
    app.add_systems(FixedUpdate, crate::collisions::resolve_hits.after(advance));
    let target = enemy(&mut app, 3, 40 * LANE_UNIT, Motion::Climb);
    let pose = *app.world().get::<Transform>(target).unwrap();
    app.world_mut()
        .spawn((crate::weapons::Bullet { lane: 3 }, pose));
    press(&mut app);
    ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Game>().score, 150);
    assert_eq!(
        app.world_mut()
            .query::<&crate::powerups::PowerUp>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(
        app.world_mut()
            .query::<&crate::weapons::Bullet>()
            .iter(app.world())
            .count(),
        1
    );
}

#[test]
fn last_chance_zaps_the_newest_eligible_enemy_regardless_of_depth() {
    for (old_depth, new_depth) in [(10, 120), (120, 10)] {
        let mut app = app();
        press(&mut app);
        ticks(&mut app, 1);
        let older = enemy(&mut app, 2, old_depth * LANE_UNIT, Motion::Climb);
        let newer = enemy(&mut app, 3, new_depth * LANE_UNIT, Motion::Climb);
        enemy(&mut app, 4, 200 * LANE_UNIT, Motion::Approach);
        shot(&mut app);
        press(&mut app);
        ticks(&mut app, 1);
        assert!(app.world().get_entity(older).is_ok());
        assert!(app.world().get_entity(newer).is_err());
        assert_eq!(app.world().resource::<Game>().score, 150);
    }
}

#[test]
fn newest_first_selection_survives_entity_slot_reuse() {
    let mut app = app();
    press(&mut app);
    ticks(&mut app, 1);
    let disposable = enemy(&mut app, 1, 30 * LANE_UNIT, Motion::Climb);
    let older = enemy(&mut app, 2, 120 * LANE_UNIT, Motion::Climb);
    app.world_mut().despawn(disposable);
    let newer = enemy(&mut app, 3, 10 * LANE_UNIT, Motion::Climb);
    assert_eq!(disposable.index(), newer.index());
    press(&mut app);
    ticks(&mut app, 1);
    assert!(app.world().get_entity(older).is_ok());
    assert!(app.world().get_entity(newer).is_err());
}

#[test]
fn second_use_hits_one_enemy_ignores_shots_and_third_use_does_nothing() {
    let mut app = app();
    press(&mut app);
    ticks(&mut app, 1); // Empty first use is still spent.
    let bullet = shot(&mut app);
    for lane in 0..2 {
        enemy(&mut app, lane, 40 * LANE_UNIT, Motion::Climb);
    }
    press(&mut app);
    ticks(&mut app, 1);
    assert!(app.world().resource::<Upgrades>().zapper.bolt.is_none());
    assert_eq!(app.world().resource::<Upgrades>().zapper.bolt_ticks, 0);
    ticks(&mut app, 19);
    assert!(app.world().get_entity(bullet).is_ok());
    assert_eq!(
        app.world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .count(),
        1
    );
    assert_eq!(app.world().resource::<Game>().score, 150);
    assert_eq!(app.world().resource::<Upgrades>().zapper.uses, 0);
    press(&mut app);
    ticks(&mut app, 8);
    assert_eq!(app.world().resource::<Game>().score, 150);
    assert_eq!(app.world().resource::<Game>().announcements.len(), 1);
}

#[test]
fn sweep_bypasses_beast_armor_and_shot_windows_but_respects_original_immunities() {
    let mut app = app();
    let beast = enemy(&mut app, 2, 80 * LANE_UNIT, Motion::Climb);
    app.world_mut().entity_mut(beast).insert(Beast { stage: 2 });
    let approaching = enemy(&mut app, 3, 200 * LANE_UNIT, Motion::Approach);
    let flipping = enemy(
        &mut app,
        4,
        0,
        Motion::Flip {
            from: 3,
            direction: 1,
            angle: 0,
        },
    );
    let fuse = app
        .world_mut()
        .spawn((
            Fuseball {
                depth: 40 * LANE_UNIT,
                edge: 5,
                motion: fuseballs::Motion::Rail,
                rise: 48,
                tick: 0,
            },
            Transform::default(),
        ))
        .id();
    let mirror = app
        .world_mut()
        .spawn((
            Mirror {
                lane: 6,
                depth: 40 * LANE_UNIT,
                approach: false,
                hits: 5,
                angle: 0,
            },
            Transform::default(),
        ))
        .id();
    let alien = |motion| AlienDroid {
        lane: 7,
        depth: 40 * LANE_UNIT,
        approach: false,
        direction: 1,
        angle: 0,
        motion,
    };
    let climbing = app
        .world_mut()
        .spawn((alien(aliens::Motion::Climb), Transform::default()))
        .id();
    let rim = app
        .world_mut()
        .spawn((alien(aliens::Motion::Ready), Transform::default()))
        .id();
    let spike = app
        .world_mut()
        .spawn((
            Spike {
                lane: 8,
                length: 30 * LANE_UNIT,
            },
            Transform::default(),
        ))
        .id();
    let reflected = shot(&mut app);
    app.world_mut()
        .entity_mut(reflected)
        .insert(Reflected { speed: 0x1c000 });
    let returned = shot(&mut app);
    app.world_mut()
        .entity_mut(returned)
        .insert(ReturnedShot { speed: 0x38000 });
    press(&mut app);
    ticks(&mut app, 40);
    for dead in [beast, flipping, fuse, rim] {
        assert!(app.world().get_entity(dead).is_err());
    }
    for alive in [approaching, mirror, climbing, spike, reflected, returned] {
        assert!(app.world().get_entity(alive).is_ok());
    }
    assert!(app.world().resource::<Game>().score >= 800);
    assert!(!app.world().resource::<Upgrades>().zapper.active);
}

#[test]
fn zapping_a_tanker_releases_its_contents_and_the_sweep_continues_through_them() {
    let mut app = app();
    app.world_mut().spawn((
        Tanker {
            lane: 3,
            depth: 80 * LANE_UNIT,
            approach: false,
        },
        Transform::default(),
    ));
    press(&mut app);
    ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Game>().score, 100);
    assert_eq!(
        app.world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .count(),
        2
    );
    ticks(&mut app, 12);
    assert_eq!(app.world().resource::<Game>().score, 400);
    assert_eq!(
        app.world_mut()
            .query::<&Flipper>()
            .iter(app.world())
            .count(),
        0
    );
    assert!(!app.world().resource::<Upgrades>().zapper.active);
}

#[test]
fn automatic_powerup_sweep_preserves_manual_charge_and_restores_a_spent_last_chance() {
    for uses in 0..=2 {
        let mut app = app();
        app.world_mut().resource_mut::<Upgrades>().zapper.uses = uses;
        enemy(&mut app, 3, 40 * LANE_UNIT, Motion::Climb);
        let mut game = Game::default();
        for _ in 0..6 {
            app.world_mut()
                .resource_mut::<Upgrades>()
                .collect(&mut game);
        }
        ticks(&mut app, 5);
        assert_eq!(app.world().resource::<Game>().score, 150);
        assert_eq!(app.world().resource::<Upgrades>().zapper.uses, uses.max(1));
        assert_eq!(app.world().resource::<Upgrades>().warp_tokens(), 1);
    }
}

#[test]
fn paused_or_nonplaying_input_cannot_spend_or_buffer_a_charge() {
    let mut app = app();
    for status in [
        Status::Entering,
        Status::Captured,
        Status::ShotHit,
        Status::Fried,
        Status::Exiting,
        Status::GameOver,
        Status::Bonus,
    ] {
        app.world_mut().resource_mut::<Game>().status = status;
        press(&mut app);
        assert!(!app.world().resource::<Upgrades>().zapper.queued);
    }
    app.world_mut().resource_mut::<Game>().status = Status::Playing;
    for lane in 0..2 {
        enemy(&mut app, lane, 40 * LANE_UNIT, Motion::Climb);
    }
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    press(&mut app);
    ticks(&mut app, 10);
    assert_eq!(app.world().resource::<Upgrades>().zapper.uses, 2);
    app.world_mut().resource_mut::<Time<Virtual>>().unpause();
    ticks(&mut app, 4);
    assert_eq!(app.world().resource::<Game>().score, 0);
    press(&mut app);
    ticks(&mut app, 1);
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    ticks(&mut app, 100);
    assert_eq!(app.world().resource::<Game>().score, 150);
    app.world_mut().resource_mut::<Time<Virtual>>().unpause();
    ticks(&mut app, 4);
    assert_eq!(app.world().resource::<Game>().score, 300);
}

#[test]
fn reentering_or_restarting_a_web_recharges_and_clears_pending_zaps() {
    for reason in [LoadReason::Retry, LoadReason::Advance, LoadReason::Restart] {
        let mut app = app();
        let mut upgrades = app.world_mut().resource_mut::<Upgrades>();
        upgrades.zapper.uses = 0;
        upgrades.zapper.active = true;
        upgrades.zapper.queued = true;
        upgrades.zapper.bolt = Some(Vec3::ONE);
        app.world_mut().resource_mut::<Game>().load = Some(reason);
        ticks(&mut app, 1);
        let zapper = &app.world().resource::<Upgrades>().zapper;
        assert_eq!(zapper.uses, 2);
        assert!(!zapper.active && !zapper.queued);
        assert!(zapper.bolt.is_none());
    }
}

#[test]
fn lightning_attaches_to_both_lane_edges_at_the_claws_current_depth() {
    for web in [
        Level::default().web,
        Web {
            closed: false,
            vertices: vec![[2., 8.], [8., 8.], [14., 8.]],
            orientations: vec![0, 0],
            start_lane: 0,
            depth: 160,
        },
    ] {
        for lane in 0..web.lanes() {
            for fraction in [0, LANE_UNIT / 4, LANE_UNIT / 2 - 1] {
                let progress = lane as i32 * LANE_UNIT + fraction;
                let z = 2.12;
                let origins = bolt_origins(&web, progress, z);
                let selected = web.selected_lane(progress);
                assert_eq!(
                    origins,
                    [
                        web.vertex(selected, FRONT_RADIUS, z),
                        web.vertex(selected + 1., FRONT_RADIUS, z)
                    ]
                );
                assert_eq!(
                    (origins[0] + origins[1]) / 2.,
                    web.glider_transform(progress).translation.with_z(z)
                );
                assert!(origins[0].distance(origins[1]) > 0.);
            }
        }
    }
}

#[test]
fn lightning_uses_the_original_lfsr_and_fixed_point_xy_noise_with_exact_endpoints() {
    let target = Vec3::new(1., 2., -12.);
    let origin = Vec3::new(4., 5., 0.12);
    let points = bolt_points(target, origin, 0);
    assert_eq!(points[0], target);
    assert_eq!(points[32], origin);
    assert!(
        points[1]
            .distance(target.lerp(origin, 1. / 32.) + Vec3::new(0.875, 0.5625, 0.) * WORLD_UNIT)
            < 1e-6
    );
    assert!(
        points[2].distance(
            target.lerp(origin, 2. / 32.) + Vec3::new(-1.28125, 1.640625, 0.) * WORLD_UNIT
        ) < 1e-6
    );
    for (i, p) in points.iter().enumerate() {
        let noise = *p - target.lerp(origin, i as f32 / 32.);
        assert!(noise.x.abs() <= 2. * WORLD_UNIT + 1e-6);
        assert!(noise.y.abs() <= 2. * WORLD_UNIT + 1e-6);
        assert_eq!(noise.z, 0.);
    }
    assert_eq!(points, bolt_points(target, origin, 1));
    assert_eq!(points, bolt_points(target, origin, 256));
    assert_ne!(points, bolt_points(target, origin, 2));
}

#[test]
fn world_space_lightning_noise_expands_near_the_camera() {
    let near = bolt_points(Vec3::new(0., 0., -20.), Vec3::new(0., 0., -5.), 0);
    let far = bolt_points(Vec3::new(0., 0., -40.), Vec3::new(0., 0., -25.), 0);
    let projection = Mat4::perspective_rh(1., 1.6, 0.1, 100.);
    assert_eq!(near[31].truncate(), far[31].truncate());
    assert!(
        projection.project_point3(near[31]).truncate().length()
            > projection.project_point3(far[31]).truncate().length() * 3.
    );
}

#[test]
fn bolt_colour_pulses_through_the_cry_palette_instead_of_holding_one_hue() {
    let cycle: std::collections::HashSet<u8> = (0..256).map(bolt_hue).collect();
    // The original flashes the whole CRY ramp, so many distinct indices appear.
    assert!(
        cycle.len() > 40,
        "only {} palette indices in one cycle",
        cycle.len()
    );
    let srgba = |frame: u32| {
        let c = crate::cry::color(bolt_hue(frame), 255).to_srgba();
        let byte = |v: f32| (v * 255.) as u8;
        (byte(c.red), byte(c.green), byte(c.blue))
    };
    let rendered: std::collections::HashSet<(u8, u8, u8)> = (0..256).map(srgba).collect();
    assert!(rendered.len() > 8, "bolt colour barely varies");
    assert!(
        rendered.iter().any(|(r, _, b)| *r > 128 && *b < 51),
        "an orange phase is expected"
    );
    assert!(
        rendered.iter().any(|(r, _, b)| *b > 77 && *r < 77),
        "a blue phase is expected"
    );
    assert_eq!(
        (0..256).map(bolt_hue).collect::<Vec<_>>(),
        (256..512).map(bolt_hue).collect::<Vec<_>>()
    );
}

#[test]
fn lightning_is_finite_shaded_vector_geometry_with_thirty_two_segments_per_bolt() {
    let target = Vec3::new(-1., 2., -20.);
    let origin = Vec3::new(3., -1., -5.);
    let projection = Mat4::perspective_rh(1., 1.6, 0.1, 100.);
    for frame in [0, 2, u32::MAX] {
        let vertices: Vec<_> = bolt_points(target, origin, frame)
            .iter()
            .map(|&p| {
                (
                    projection.project_point3(p).truncate(),
                    crate::cry::color(bolt_hue(frame), 160),
                )
            })
            .collect();
        let triangles = bolt_triangles(&vertices);
        assert_eq!(triangles.len(), 64);
        assert!(triangles.iter().flatten().all(|(p, _)| p.is_finite()));
        assert!(
            ((triangles[0][0].0 + triangles[0][1].0) / 2.)
                .truncate()
                .distance(vertices[0].0)
                < 1e-6
        );
        let last = triangles.last().unwrap();
        assert!(
            ((last[1].0 + last[2].0) / 2.)
                .truncate()
                .distance(vertices[32].0)
                < 1e-6
        );
        assert!(triangles.iter().flatten().all(|(_, c)| *c == vertices[0].1));
    }
    // gamefx: high nibble from sine[frames], low nibble from sine[frames + 0x40].
    assert_eq!(
        [bolt_hue(0), bolt_hue(64), bolt_hue(128), bolt_hue(192)],
        [0x8f, 0xf8, 0x80, 0x08]
    );
    assert_eq!(bolt_hue(256), bolt_hue(0));
    assert!(!matches!(bolt_hue(0), 0) && !matches!(bolt_hue(0), 255));
}
