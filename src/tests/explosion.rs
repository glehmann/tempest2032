//! Checks original explosion sine samples, hue changes, ring geometry, and screen-sized dots.

use crate::explosion::BurstAssets;
use crate::explosion::DOT_SIZE;
use crate::explosion::Explosion;
use crate::explosion::Frames;
use crate::explosion::MAX_AGE;
use crate::explosion::POINTS;
use crate::explosion::RING_DEPTH;
use crate::explosion::RING_SCALE;
use crate::explosion::SINES;
use crate::explosion::WORLD_SCALE;
use crate::explosion::age_explosions;
use crate::explosion::burst_colour;
use crate::explosion::burst_mesh;
use crate::explosion::pulse_hue;
use crate::explosion::resize_dots;
use crate::explosion::ring_triangles;
use crate::explosion::spawn_explosion;
use bevy::prelude::*;

#[test]
fn distance_compensation_enlarges_dots_without_moving_the_ring() {
    let mut mesh = burst_mesh(25);
    let positions = |mesh: &Mesh| {
        let bevy::mesh::VertexAttributeValues::Float32x3(values) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!()
        };
        values.clone()
    };
    let before = positions(&mesh);
    resize_dots(&mut mesh, 4.0);
    let after = positions(&mesh);
    for (before, after) in before
        .as_chunks::<6>()
        .0
        .iter()
        .zip(after.as_chunks::<6>().0)
    {
        let centre =
            |quad: &[[f32; 3]; 6]| (Vec3::from_array(quad[0]) + Vec3::from_array(quad[2])) * 0.5;
        assert!(centre(before).distance(centre(after)) < 1e-5);
        let width =
            |quad: &[[f32; 3]; 6]| Vec3::from_array(quad[0]).distance(Vec3::from_array(quad[1]));
        assert!((width(after) - width(before) * 4.0).abs() < 1e-5);
    }
}

fn rgb(colour: Color) -> [f32; 3] {
    let [r, g, b, _] = LinearRgba::from(colour).to_f32_array();
    [r, g, b]
}

#[test]
fn the_sine_table_matches_the_original() {
    assert_eq!(SINES.len(), 256);
    assert_eq!(SINES[0], 0);
    assert_eq!(SINES[16], 48);
    assert_eq!(SINES[64], 126);
    assert_eq!(SINES[128], 0);
    assert_eq!(SINES[192], -126);
    assert_eq!(*SINES.iter().max().unwrap(), 126);
}

#[test]
fn the_hue_takes_one_nibble_from_each_quarter_turn_of_the_sine_table() {
    for counter in [0u8, 1, 33, 64, 200] {
        let hue = pulse_hue(counter);
        assert_eq!(
            hue >> 4,
            ((SINES[counter as usize] as i32 + 0x80) as u8 & 0xf0) >> 4
        );
        assert_eq!(
            hue & 0x0f,
            ((SINES[counter.wrapping_add(0x40) as usize] as i32 + 0x80) >> 4) as u8 & 0x0f
        );
    }
}

#[test]
fn the_two_rings_sweep_the_sine_table_from_green_into_orange() {
    // draw_prex hands the inner ring its own radius as the colour index and
    // the outer ring twice that.
    let inner = rgb(burst_colour(1));
    let inner_end = rgb(burst_colour(MAX_AGE));
    assert!(
        inner[0] < 0.2 && inner[1] > 0.9 && inner[2] < 0.05,
        "the inner ring starts green {inner:?}"
    );
    assert!(
        inner_end[0] > inner[0] && inner_end[1] > 0.9,
        "the inner ring only reaches lime {inner_end:?}"
    );
    let outer_end = rgb(burst_colour(2 * MAX_AGE));
    assert!(
        outer_end[0] > 0.9 && outer_end[1] < 0.4 && outer_end[2] < 0.05,
        "the outer ring carries on into orange {outer_end:?}"
    );
}

#[test]
fn rings_use_the_original_point_counts_and_growth() {
    const { assert!(RING_DEPTH[1] < 0.0) };
    assert_eq!(POINTS, [16, 14]);
    assert_eq!(RING_SCALE, [1.0, 2.0]);
    assert_eq!(RING_DEPTH[0], 0.0);
    assert_eq!(ring_triangles(0, MAX_AGE).len(), POINTS[0] * 2);
    assert_eq!(ring_triangles(1, MAX_AGE).len(), POINTS[1] * 2);
}

#[test]
fn every_frame_of_the_explosion_is_prebuilt() {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<BurstAssets>()
        .init_resource::<Frames>()
        .init_resource::<Time>()
        .add_systems(FixedUpdate, age_explosions);
    let assets = BurstAssets::default();
    let mut commands = app.world_mut().commands();
    spawn_explosion(&mut commands, &assets, Vec3::new(1.0, 2.0, -0.5));
    app.world_mut().flush();
    let live = |app: &mut App| {
        app.world_mut()
            .query::<&Explosion>()
            .iter(app.world())
            .count()
    };
    assert_eq!(live(&mut app), 1);
    for _ in 0..MAX_AGE {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert_eq!(live(&mut app), 1, "still alive on its last tick");
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(live(&mut app), 0);
}

#[test]
fn the_dots_keep_one_size_while_the_rings_grow() {
    // Each frame has its own mesh, which is what stops the dots from
    // stretching with the ring the way the original plots single pixels.
    let dot = |age: u8| {
        let triangle = ring_triangles(0, age)[0];
        (triangle[1].0 - triangle[0].0).length()
    };
    // A dot at angle zero straddles the radius along the x axis, so the midpoint of
    // its x span is exactly the radius it was drawn at.
    let radius = |age: u8| {
        let triangle = ring_triangles(0, age)[0];
        let (low, high) = triangle
            .iter()
            .map(|(point, _)| point.x)
            .fold((f32::MAX, f32::MIN), |(low, high), x| {
                (low.min(x), high.max(x))
            });
        (low + high) * 0.5
    };
    for age in [1u8, 13, 27, MAX_AGE] {
        assert!(
            (dot(age) - 2.0 * DOT_SIZE).abs() < 1e-5,
            "a dot is {} wide at age {age}",
            dot(age)
        );
        assert!(
            (radius(age) - age as f32 * WORLD_SCALE).abs() < 1e-5,
            "age {age} drew a ring of {}",
            radius(age)
        );
    }
}
