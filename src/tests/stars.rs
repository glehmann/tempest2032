//! Checks star projection, depth wrapping, brightness, population, and warp trail samples.

use crate::mesh::colored_mesh;
use crate::stars::COUNT;
use crate::stars::Star;
use crate::stars::StarField;
use crate::stars::Stars;
use crate::stars::Warp;
use crate::stars::draw_stars;
use crate::web::LANE_UNIT;
use bevy::prelude::*;

#[test]
fn warp_draws_four_depth_spaced_samples_and_returns_to_single_dots() {
    let mut app = App::new();
    let mut meshes = Assets::<Mesh>::default();
    let mesh = meshes.add(colored_mesh([]));
    app.insert_resource(meshes)
        .insert_resource(Stars(vec![Star {
            position: Vec2::ZERO,
            depth: 32 * LANE_UNIT,
            color: Color::WHITE,
        }]))
        .insert_resource(Warp {
            distance: 0,
            spacing: 16 * LANE_UNIT,
            copies: 3,
        })
        .add_systems(Update, draw_stars);
    app.world_mut().spawn((
        Camera3d::default(),
        Transform::default(),
        Projection::Perspective(PerspectiveProjection::default()),
    ));
    app.world_mut()
        .spawn((StarField, Mesh3d(mesh.clone()), Transform::default()));
    app.world_mut().run_schedule(Update);
    let colors = match app
        .world()
        .resource::<Assets<Mesh>>()
        .get(&mesh)
        .unwrap()
        .attribute(Mesh::ATTRIBUTE_COLOR)
        .unwrap()
    {
        bevy::mesh::VertexAttributeValues::Float32x4(colors) => colors,
        _ => panic!("star mesh needs colors"),
    };
    assert_eq!(colors.len(), 24);
    assert!(
        colors
            .chunks_exact(6)
            .map(|dot| dot[0][0])
            .collect::<Vec<_>>()
            .windows(2)
            .all(|pair| pair[0] > pair[1])
    );
    *app.world_mut().resource_mut::<Warp>() = Warp::default();
    app.world_mut().run_schedule(Update);
    assert_eq!(
        app.world()
            .resource::<Assets<Mesh>>()
            .get(&mesh)
            .unwrap()
            .count_vertices(),
        6
    );
}

fn star(depth: i32) -> Star {
    Star {
        position: Vec2::new(30.0, -20.0),
        depth,
        color: Color::WHITE,
    }
}

#[test]
fn stars_accelerate_outward_and_brighten_as_the_viewpoint_advances() {
    let mut star = star(100 * LANE_UNIT);
    let before = star.projected(Vec2::ZERO);
    let brightness = star.color().to_srgba().red;
    star.advance();
    let first_step = star.projected(Vec2::ZERO) - before;
    assert!(star.projected(Vec2::ZERO).length() > before.length());
    assert!(star.color().to_srgba().red > brightness);
    let before = star.projected(Vec2::ZERO);
    star.advance();
    assert!((star.projected(Vec2::ZERO) - before).length() > first_step.length());
}

#[test]
fn near_stars_wrap_back_into_the_distance_without_changing_their_ray() {
    let mut star = star(LANE_UNIT);
    let direction = star.projected(Vec2::ZERO).normalize();
    star.advance();
    assert_eq!(star.depth, 511 * LANE_UNIT);
    assert!(star.projected(Vec2::ZERO).normalize().distance(direction) < 1e-6);
    assert!(star.projected(Vec2::ZERO).is_finite());
}

#[test]
fn motion_repeats_after_256_ticks_without_changing_the_population() {
    let mut stars = Stars::default();
    let initial: Vec<_> = stars.0.iter().map(|star| star.depth).collect();
    for _ in 0..256 {
        for star in &mut stars.0 {
            star.advance();
        }
    }
    assert_eq!(stars.0.len(), COUNT);
    assert_eq!(
        stars.0.iter().map(|star| star.depth).collect::<Vec<_>>(),
        initial
    );
}
