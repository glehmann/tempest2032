//! Checks shot colors, particle-ring sizing, firing cadence, magazine limits, and lane retention.

use crate::powerups::Upgrades;
use crate::test_support::START_PROGRESS;
use crate::test_support::WEB_PERIOD;
use crate::test_support::bullet_count;
use crate::test_support::firing_app;
use crate::test_support::glider_transform;
use crate::test_support::selected_lane;
use crate::test_support::state;
use crate::test_support::ticks;
use crate::weapons::BULLET_STEP;
use crate::weapons::Bullet;
use crate::weapons::MAX_BULLETS;
use crate::weapons::Weapon;
use crate::weapons::particle_shot_mesh;
use crate::weapons::shot_mesh;
use crate::web::FRONT_RADIUS;
use bevy::prelude::*;
use std::time::Duration;

#[test]
fn particle_rings_have_four_separate_screen_sized_dots_and_change_color_with_depth() {
    let camera = Transform::from_translation(Vec3::new(0.0, 0.0, 5.3))
        .with_rotation(Quat::from_rotation_y(0.1));
    let projection = PerspectiveProjection {
        fov: 1.2,
        aspect_ratio: 1.294,
        ..default()
    };
    let mut shades = Vec::new();
    for z in [0.0, -10.0] {
        let position = Vec3::new(0.0, 0.0, z);
        let mesh = particle_shot_mesh(position, &camera, &projection);
        let vertices = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();
        assert_eq!(vertices.len(), 24);
        for dot in vertices.chunks_exact(6) {
            let local = |index| camera.rotation.inverse() * Vec3::from_array(dot[index]);
            let width = (local(1) - local(0)).length();
            let distance = -(camera.rotation.inverse() * (position - camera.translation)).z;
            let pixels = width * 384.0
                / (2.0 * distance * (projection.fov * 0.5).tan() * projection.aspect_ratio);
            assert!((pixels - 2.0).abs() < 1e-4);
            let center = (local(0) + local(2)) * 0.5;
            assert!((center.length() - 4.0 * FRONT_RADIUS / 24.0).abs() < 1e-5);
        }
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("missing particle colors");
        };
        shades.push(colors[0]);
    }
    assert_ne!(shades[0], shades[1]);
}

#[test]
fn shot_has_a_white_core_inside_a_cyan_green_glow() {
    let mesh = shot_mesh();
    let colors = match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
        Some(bevy::mesh::VertexAttributeValues::Float32x4(values)) => values,
        other => panic!("shot mesh needs colours, got {other:?}"),
    };
    assert!(!colors.is_empty());

    // Exactly the three sampled shades, in the linear space the GPU uses.
    let shade = |rgb: [u8; 3]| {
        let value = LinearRgba::from(Color::srgb_u8(rgb[0], rgb[1], rgb[2])).to_f32_array();
        [value[0], value[1], value[2]]
    };
    let shades = [
        shade([236, 250, 254]),
        shade([14, 250, 155]),
        shade([0, 154, 53]),
    ];
    let matches = |color: &[f32], want: &[f32; 3]| {
        (0..3).all(|channel| (color[channel] - want[channel]).abs() < 1e-6)
    };
    for color in colors {
        assert!(
            shades.iter().any(|want| matches(color, want)),
            "unexpected shot colour {color:?}"
        );
    }
    for want in shades {
        assert!(colors.iter().any(|color| matches(color, &want)));
    }
    // The core is the brightest layer and the rim the dimmest.
    let luminance = |shade: &[f32; 3]| shade.iter().sum::<f32>();
    assert!(luminance(&shades[0]) > luminance(&shades[1]));
    assert!(luminance(&shades[1]) > luminance(&shades[2]));
}

#[test]
fn z_fires_immediately_and_repeats_at_the_weapon_cadence() {
    let mut app = firing_app(START_PROGRESS, &[]);
    ticks(&mut app, 1);
    assert_eq!(bullet_count(&mut app), 0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyZ);
    ticks(&mut app, 1);
    assert_eq!(bullet_count(&mut app), 1);
    ticks(&mut app, 3);
    assert_eq!(bullet_count(&mut app), 1);
    ticks(&mut app, 1);
    assert_eq!(bullet_count(&mut app), 2);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyZ);
    ticks(&mut app, 8);
    assert_eq!(bullet_count(&mut app), 2);
    assert!(
        !app.world_mut()
            .query::<&Weapon>()
            .single(app.world())
            .unwrap()
            .firing
    );
}

#[test]
fn a_full_magazine_skips_a_shot_instead_of_firing_as_soon_as_a_slot_frees() {
    let mut app = firing_app(START_PROGRESS, &[KeyCode::KeyZ]);
    let slots: Vec<_> = (0..MAX_BULLETS)
        .map(|_| {
            app.world_mut()
                .spawn((Bullet { lane: 7 }, Transform::default()))
                .id()
        })
        .collect();
    ticks(&mut app, 1);
    app.world_mut().despawn(slots[0]);
    ticks(&mut app, 3);
    assert_eq!(bullet_count(&mut app), MAX_BULLETS - 1);
    assert!(
        !app.world_mut()
            .query::<&Weapon>()
            .single(app.world())
            .unwrap()
            .firing
    );
    ticks(&mut app, 1);
    assert_eq!(bullet_count(&mut app), MAX_BULLETS);
    assert!(
        app.world_mut()
            .query::<&Weapon>()
            .single(app.world())
            .unwrap()
            .firing
    );
}

#[test]
fn menu_confirm_jump_and_zapper_keys_do_not_fire() {
    for key in [
        KeyCode::Enter,
        KeyCode::NumpadEnter,
        KeyCode::KeyX,
        KeyCode::KeyC,
    ] {
        let mut app = firing_app(START_PROGRESS, &[key]);
        ticks(&mut app, 8);
        assert_eq!(bullet_count(&mut app), 0, "{key:?}");
    }
}

#[test]
fn particle_laser_increases_fire_rate_and_shot_speed() {
    let mut app = firing_app(START_PROGRESS, &[KeyCode::Space]);
    let mut upgrades = Upgrades::default();
    upgrades.laser = true;
    app.insert_resource(upgrades);
    ticks(&mut app, 1);
    let shot = app
        .world_mut()
        .query_filtered::<Entity, With<Bullet>>()
        .single(app.world())
        .unwrap();
    let start = app.world().get::<Transform>(shot).unwrap().translation.z;
    ticks(&mut app, 3);
    assert_eq!(bullet_count(&mut app), 2);
    let position = app.world().get::<Transform>(shot).unwrap().translation.z;
    assert!((start - position - 3.0 * 4.5 * FRONT_RADIUS / 24.0).abs() < 1e-5);
}

#[test]
fn shots_keep_their_launch_lane_while_the_claw_moves_and_wraps() {
    let mut app = firing_app(WEB_PERIOD - 512, &[KeyCode::Space, KeyCode::ArrowLeft]);
    ticks(&mut app, 1);
    let shot = app
        .world_mut()
        .query_filtered::<Entity, With<Bullet>>()
        .single(app.world())
        .unwrap();
    assert_eq!(app.world().get::<Bullet>(shot).unwrap().lane, 0);
    let start = app.world().get::<Transform>(shot).unwrap().translation;
    assert_eq!(start.truncate(), glider_transform(0).translation.truncate());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::Space);
    ticks(&mut app, 16);
    assert_ne!(selected_lane(state(&mut app).0), 0.0);
    let position = app.world().get::<Transform>(shot).unwrap().translation;
    assert_eq!(position.truncate(), start.truncate());
    assert!((position.z - (start.z - 16.0 * BULLET_STEP)).abs() < 1e-4);
}

#[test]
fn sustained_fire_is_bounded_and_shots_are_cleaned_up() {
    let mut app = firing_app(START_PROGRESS, &[KeyCode::Space]);
    let mut reached_limit = false;
    for _ in 0..180 {
        ticks(&mut app, 1);
        let count = bullet_count(&mut app);
        assert!(count <= MAX_BULLETS);
        reached_limit |= count == MAX_BULLETS;
    }
    assert!(reached_limit);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::Space);
    ticks(&mut app, 60);
    assert_eq!(bullet_count(&mut app), 0);
    assert_eq!(
        app.world_mut().query::<&Mesh3d>().iter(app.world()).count(),
        0
    );
}

#[test]
fn repeat_fire_is_independent_of_render_frame_rate() {
    use bevy::time::TimeUpdateStrategy;

    let mut slow = firing_app(0, &[KeyCode::Space]);
    let mut fast = firing_app(0, &[KeyCode::Space]);
    for (app, nanos) in [(&mut slow, 33_333_334), (&mut fast, 16_666_667)] {
        app.add_plugins(MinimalPlugins)
            .insert_resource(Time::<Fixed>::from_hz(60.0))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_nanos(
                nanos,
            )));
        app.update();
    }
    for _ in 0..30 {
        slow.update();
    }
    for _ in 0..60 {
        fast.update();
    }
    let positions = |app: &mut App| {
        let mut depths: Vec<_> = app
            .world_mut()
            .query_filtered::<&Transform, With<Bullet>>()
            .iter(app.world())
            .map(|transform| transform.translation.z)
            .collect();
        depths.sort_by(f32::total_cmp);
        depths
    };
    assert_eq!(positions(&mut slow), positions(&mut fast));
}
