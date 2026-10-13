//! Independent, real-time stars behind the front-end panels.

use super::{FrontEnd, Screen};
use bevy::prelude::*;

#[derive(Component)]
pub(super) struct Canvas;

#[derive(Component)]
struct Star(crate::stars::Star);

pub(super) fn add_systems(app: &mut App) {
    app.add_systems(Startup, setup.after(super::setup))
        .add_systems(Update, advance);
}

fn setup(mut commands: Commands, canvas: Single<Entity, With<Canvas>>) {
    commands.entity(*canvas).with_children(|canvas| {
        for star in crate::stars::Pattern::Level.stars(2) {
            canvas.spawn((
                Star(star),
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                BackgroundColor(Color::BLACK),
            ));
        }
    });
}

fn advance(
    front: Res<FrontEnd>,
    time: Res<Time<Real>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut fraction: Local<f32>,
    mut stars: Query<(&mut Star, &mut Node, &mut BackgroundColor)>,
) {
    if front.screen == Screen::Gameplay {
        return;
    }
    let Some(window) = windows.iter().next() else {
        return;
    };
    // Limit motion after suspension, without advancing the gameplay star field.
    *fraction += time.delta_secs().min(0.1) * 60. * crate::stars::STEP as f32;
    let step = *fraction as i32;
    *fraction -= step as f32;
    let scale = window.height() / 240.;
    let size = scale.max(1.);
    for (mut star, mut node, mut color) in &mut stars {
        star.0.advance(step);
        let position =
            star.0.projected(Vec2::ZERO) * scale + Vec2::new(window.width(), window.height()) / 2.;
        node.left = px(position.x - size / 2.);
        node.top = px(position.y - size / 2.);
        node.width = px(size);
        node.height = px(size);
        color.0 = star.0.color();
    }
}
