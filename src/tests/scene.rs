//! Checks web fill palettes, animated shading, and highlighted lane edges.

use crate::glider::Glider;
use crate::glider::move_glider;
use crate::scene::LaneHighlight;
use crate::scene::update_lane;
use crate::test_support::DEPTH;
use crate::test_support::glider_transform;
use crate::test_support::movement_app;
use crate::test_support::rim_vertex;
use crate::test_support::ticks;
use crate::web::BACK_RADIUS;
use crate::web::FRONT_RADIUS;
use crate::web::LANE_UNIT;
use bevy::prelude::*;

#[test]
fn every_bundled_web_has_an_explicit_valid_surface_palette() {
    let levels = crate::test_support::levels();
    for level in &levels.0 {
        let hues = level.appearance.surface_hues.as_ref().unwrap();
        assert_eq!(hues.is_empty(), (49..=64).contains(&level.number));
        if !hues.is_empty() {
            assert_eq!(hues.len(), level.web.lanes());
        }
        let mesh = super::tunnel_mesh(&level.web, &level.appearance);
        assert_eq!(
            mesh.count_vertices(),
            if hues.is_empty() {
                0
            } else {
                level.web.lanes() * super::SURFACE_SECTIONS * 6
            }
        );
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("missing fill colors")
        };
        assert!(colors.iter().flatten().all(|value| value.is_finite()));
    }
}

#[test]
fn level_five_uses_the_triangles_blue_purple_lane_colors_independently_of_the_wires() {
    let level = crate::level::Level::parse(include_str!("../../levels/05.toml"), "level5").unwrap();
    let hues = level.appearance.surface_hues.as_ref().unwrap();
    assert_eq!(
        hues,
        &[
            0x04, 0x14, 0x23, 0x33, 0x42, 0x52, 0x61, 0x71, 0x70, 0x70, 0x71, 0x61, 0x52, 0x42,
            0x33, 0x23, 0x14, 0x04
        ]
    );
    assert_eq!(
        super::lane_color(hues, 0, 0., 0),
        Color::srgb_u8(0, 47, 176)
    );
    assert_eq!(
        super::lane_color(hues, 3, 0., 0),
        Color::srgb_u8(48, 33, 120)
    );
    assert_eq!(
        super::lane_color(hues, 3, 1., 0),
        crate::cry::color(0x33, 30)
    );
    let mut appearance = level.appearance.clone();
    appearance.surface = [1., 0., 0.];
    appearance.lines = [0., 1., 0.];
    assert_eq!(
        super::surface_color(&level.web, &appearance, 3, 0., 0),
        super::surface_color(&level.web, &level.appearance, 3, 0., 0)
    );
}

#[test]
fn surface_shading_cycles_without_changing_geometry_or_allocating_meshes() {
    let level = crate::level::Level::parse(include_str!("../../levels/05.toml"), "level5").unwrap();
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>()
        .add_systems(FixedUpdate, super::animate_surface);
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(super::tunnel_mesh(&level.web, &level.appearance));
    let positions = app
        .world()
        .resource::<Assets<Mesh>>()
        .get(&handle)
        .unwrap()
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .clone();
    app.world_mut().spawn((
        Mesh3d(handle.clone()),
        super::WebSurface {
            hues: level.appearance.surface_hues.unwrap(),
            depth: level.web.world_depth(),
        },
    ));
    let colors = |app: &App| {
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) = app
            .world()
            .resource::<Assets<Mesh>>()
            .get(&handle)
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("missing colors")
        };
        (colors.as_ptr(), colors.clone())
    };
    let initial = colors(&app);
    ticks(&mut app, 64);
    let changed = colors(&app);
    assert_eq!(
        changed.0, initial.0,
        "surface color storage should be reused"
    );
    assert_ne!(changed.1, initial.1);
    ticks(&mut app, 256);
    assert_eq!(colors(&app), changed);
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 1);
    let bevy::mesh::VertexAttributeValues::Float32x3(expected) = positions else {
        panic!("missing positions")
    };
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(actual)) = app
        .world()
        .resource::<Assets<Mesh>>()
        .get(&handle)
        .unwrap()
        .attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("missing positions")
    };
    assert_eq!(actual, &expected);
}

#[test]
fn lane_highlights_switch_with_the_claw() {
    let mut app = movement_app(LANE_UNIT / 2 - 512, &[KeyCode::KeyA]);
    app.add_systems(FixedUpdate, update_lane.after(move_glider));
    let edges = [0, 1].map(|edge| {
        app.world_mut()
            .spawn((LaneHighlight(edge), Transform::default()))
            .id()
    });
    ticks(&mut app, 1);
    let glider_position = app
        .world_mut()
        .query_filtered::<&Transform, With<Glider>>()
        .single(app.world())
        .unwrap()
        .translation;
    assert_eq!(glider_position, glider_transform(LANE_UNIT).translation);
    for (edge, entity) in edges.into_iter().enumerate() {
        let expected = (rim_vertex(1.0 + edge as f32, FRONT_RADIUS, 0.0)
            + rim_vertex(1.0 + edge as f32, BACK_RADIUS, -DEPTH))
            * 0.5
            + Vec3::Z * 0.025;
        assert!(
            app.world()
                .get::<Transform>(entity)
                .unwrap()
                .translation
                .distance(expected)
                < 1e-5
        );
    }
}
