use super::*;

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<Glyphs>()
        .init_resource::<super::super::font::Font>()
        .init_resource::<Messages>()
        .init_resource::<Time<Virtual>>()
        .init_resource::<crate::pause::Menu>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .add_systems(Startup, setup)
        .add_systems(Update, draw);
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    app.update();
    app
}

#[test]
fn paused_word_is_gold_and_red_geometry_and_has_no_mouse_controls_or_dimmed_panel() {
    let glyphs = Glyphs::default();
    assert!(paused_geometry(&glyphs, 384., false).is_empty());
    let triangles = paused_geometry(&glyphs, 384., true);
    assert!(triangles.len() > 100);
    assert!(
        triangles
            .iter()
            .flatten()
            .all(|(p, c)| p.is_finite() && c.to_linear().alpha == 1.)
    );
    assert!(triangles.iter().flatten().any(|(_, c)| {
        let c = c.to_srgba();
        c.red > 0.4 && c.green < 0.1 && c.blue < 0.1
    }));
    assert!(triangles.iter().flatten().any(|(_, c)| {
        let c = c.to_srgba();
        c.red > 0.6 && c.green > 0.3 && c.blue < 0.3
    }));
    let mut app = app();
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    app.update();
    let handle = &app.world().resource::<Graphics>().pause;
    assert!(
        app.world()
            .resource::<Assets<Mesh>>()
            .get(handle)
            .unwrap()
            .count_vertices()
            > 0
    );
    assert_eq!(
        app.world_mut().query::<&Node>().iter(app.world()).count(),
        0
    );
    assert_eq!(
        app.world_mut().query::<&Button>().iter(app.world()).count(),
        0
    );
    app.world_mut().resource_mut::<Time<Virtual>>().unpause();
    app.update();
    let handle = &app.world().resource::<Graphics>().pause;
    assert_eq!(
        app.world()
            .resource::<Assets<Mesh>>()
            .get(handle)
            .unwrap()
            .count_vertices(),
        0
    );
}

#[test]
fn pause_selection_redraws_even_while_the_message_clock_is_frozen() {
    let mut app = app();
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    app.update();
    let handle = app.world().resource::<Graphics>().pause.clone();
    let before = app
        .world()
        .resource::<Assets<Mesh>>()
        .get(&handle)
        .unwrap()
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .clone();
    app.world_mut()
        .resource_mut::<crate::pause::Menu>()
        .selected = 1;
    app.update();
    let after = app
        .world()
        .resource::<Assets<Mesh>>()
        .get(&handle)
        .unwrap()
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap();
    assert_ne!(&before, after);
    assert_eq!(app.world().resource::<Messages>().clock, 0);
}

#[test]
fn sharp_banner_has_geometry_and_disappears_when_replaced_without_growing_asset_count() {
    let mut app = app();
    for id in [Id::ParticleLaser, Id::SuperzapperRecharge, Id::GreenTrack] {
        app.world_mut().resource_mut::<Messages>().banner = Some(super::super::Active {
            event: id.into(),
            age: 60,
        });
        app.update();
        let handle = app.world().resource::<Graphics>().banner.clone();
        assert!(
            app.world()
                .resource::<Assets<Mesh>>()
                .get(&handle)
                .unwrap()
                .count_vertices()
                > 0
        );
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        let mesh = meshes.get_mut(&handle).unwrap();
        assert_eq!(
            mesh.asset_usage,
            bevy::asset::RenderAssetUsages::RENDER_WORLD
        );
        mesh.take_gpu_data().unwrap();
    }
    app.world_mut().resource_mut::<Messages>().banner = None;
    app.update();
    let handle = &app.world().resource::<Graphics>().banner;
    assert_eq!(
        app.world()
            .resource::<Assets<Mesh>>()
            .get(handle)
            .unwrap()
            .count_vertices(),
        0
    );
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 2);
    assert_eq!(app.world().resource::<Assets<StandardMaterial>>().len(), 1);
}

#[test]
fn accepted_approach_effects_are_not_duplicated_in_the_new_renderer() {
    let mut app = app();
    for id in [
        Id::Points2000,
        Id::Excellent,
        Id::OneUp,
        Id::Points250,
        Id::Points500,
        Id::Points750,
    ] {
        app.world_mut().resource_mut::<Messages>().rewards = vec![super::super::Active {
            event: id.into(),
            age: 30,
        }];
        app.update();
        assert!(
            app.world()
                .resource::<Assets<Mesh>>()
                .iter()
                .all(|(_, mesh)| mesh.count_vertices() == 0)
        );
    }
}

#[test]
fn game_over_uses_the_previous_bitmap_feedback_without_a_sharp_3d_foreground() {
    let mut app = app();
    for age in [0, 4, 120] {
        app.world_mut().resource_mut::<Messages>().game_over = Some(super::super::Active {
            event: Id::GameOver.into(),
            age,
        });
        app.update();
        for (_, mesh) in app.world().resource::<Assets<Mesh>>().iter() {
            assert_eq!(mesh.count_vertices(), 0);
        }
    }
    app.world_mut().resource_mut::<Messages>().game_over = None;
    app.update();
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 2);
}

#[test]
fn overlay_camera_is_not_moved_by_gameplay_camera_tracking() {
    let mut app = app();
    app.init_resource::<crate::level::Level>()
        .add_systems(FixedUpdate, crate::camera::follow_glider);
    app.world_mut().spawn((
        crate::glider::Glider::default(),
        Transform::from_xyz(3., 4., 0.12),
    ));
    let camera = app
        .world_mut()
        .query_filtered::<Entity, With<OverlayCamera>>()
        .single(app.world())
        .unwrap();
    let before = *app.world().get::<Transform>(camera).unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(*app.world().get::<Transform>(camera).unwrap(), before);
}

#[test]
#[ignore = "manual CPU 3D message rendering benchmark"]
fn benchmark_recharge_rendering() {
    let mut app = app();
    app.world_mut().resource_mut::<Messages>().banner = Some(super::super::Active {
        event: Id::SuperzapperRecharge.into(),
        age: 280,
    });
    app.update();
    let start = std::time::Instant::now();
    for _ in 0..200 {
        app.world_mut().resource_mut::<Messages>().clock += 1;
        app.update();
    }
    let handle = &app.world().resource::<Graphics>().banner;
    eprintln!(
        "3D message mesh: {} vertices; mean CPU draw: {:.3} ms",
        app.world()
            .resource::<Assets<Mesh>>()
            .get(handle)
            .unwrap()
            .count_vertices(),
        start.elapsed().as_secs_f64() * 1000. / 200.
    );
}
