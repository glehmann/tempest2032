//! Checks Flipper spawn budgets, climb speed, rim flips, approach dots, and facet colors.

use crate::enemies::flipper_render::approach_dot_style;
use crate::enemies::flipper_render::flipper_mesh;
use crate::enemies::flippers::Flipper;
use crate::enemies::flippers::Motion;
use crate::enemies::flippers::flipper_transform;
use crate::enemies::flippers::move_enemies;
use crate::enemies::flippers::spawn_enemies;
use crate::enemies::flippers::start_flip;
use crate::game::Game;
use crate::game::Status;
use crate::game::reset_wave;
use crate::level::Level;
use crate::test_support::BRIGHTEST_RED;
use crate::test_support::MAX_FLIPPERS;
use crate::test_support::REAR;
use crate::test_support::app;
use crate::test_support::enemy;
use crate::test_support::ticks;
use crate::web::LANE_UNIT;
use crate::web::WORLD_UNIT;
use bevy::prelude::*;

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
        let level = Level::parse(
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/02.toml")),
            "level2",
        )
        .unwrap();
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
    let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/02.toml"))
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
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/02.toml")),
        "level2",
    )
    .unwrap();
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
