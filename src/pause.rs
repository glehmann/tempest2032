//! Pause input and UI stay live while virtual time, fixed updates, and audio stop.

use bevy::{
    app::{RunFixedMainLoop, RunFixedMainLoopSystems},
    prelude::*,
};

#[derive(Component)]
struct PauseButton;

#[derive(Component)]
struct ButtonLabel;

#[derive(Component)]
struct Overlay;

pub(crate) fn running(time: Option<Res<Time<Virtual>>>) -> bool {
    time.is_none_or(|time| !time.is_paused())
}

pub(crate) fn add_systems(app: &mut App) {
    app.configure_sets(
        RunFixedMainLoop,
        RunFixedMainLoopSystems::FixedMainLoop.run_if(running),
    )
    .add_systems(Startup, setup)
    .add_systems(
        PreUpdate,
        (input, audio)
            .chain()
            .after(bevy::input::InputSystems)
            .after(bevy::ui::UiSystems::Focus)
            .before(crate::powerups::queue_jump),
    )
    .add_systems(Update, draw);
}

fn setup(mut commands: Commands) {
    commands
        .spawn((
            PauseButton,
            Button,
            Node {
                position_type: PositionType::Absolute,
                right: px(26),
                top: px(80),
                padding: UiRect::axes(px(14), px(9)),
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.06, 0.07, 0.12)),
            BorderColor::all(Color::srgb(0.6, 0.5, 0.1)),
            GlobalZIndex(301),
        ))
        .with_children(|parent| {
            parent.spawn((
                ButtonLabel,
                Text::new("PAUSE [P]"),
                TextFont {
                    font_size: 16.,
                    ..default()
                },
                TextColor(Color::srgb(1., 0.85, 0.15)),
            ));
        });
    commands
        .spawn((
            Overlay,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                display: Display::None,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(16),
                ..default()
            },
            BackgroundColor(Color::BLACK.with_alpha(0.55)),
            GlobalZIndex(300),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("PAUSED"),
                TextFont {
                    font_size: 48.,
                    ..default()
                },
                TextColor(Color::srgb(1., 0.85, 0.15)),
            ));
            parent.spawn((
                Text::new("Press P or click Resume to continue"),
                TextFont {
                    font_size: 18.,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.85)),
            ));
        });
}

fn input(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Query<&Interaction, (With<PauseButton>, Changed<Interaction>)>,
    mut time: ResMut<Time<Virtual>>,
    mut upgrades: Option<ResMut<crate::powerups::Upgrades>>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        time.unpause();
    } else if keys.any_just_pressed([KeyCode::KeyP, KeyCode::Pause])
        || buttons
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed)
    {
        if time.is_paused() {
            time.unpause();
        } else {
            time.pause();
        }
    } else {
        return;
    }
    if let Some(upgrades) = &mut upgrades {
        upgrades.jump_queued = false;
    }
}

fn audio(
    time: Res<Time<Virtual>>,
    mut players: Query<(&mut PlaybackSettings, Option<&AudioSink>)>,
) {
    let paused = time.is_paused();
    for (mut settings, sink) in &mut players {
        if settings.paused != paused {
            settings.paused = paused;
        }
        if let Some(sink) = sink {
            if paused && !sink.is_paused() {
                sink.pause();
            } else if !paused && sink.is_paused() {
                sink.play();
            }
        }
    }
}

fn draw(
    time: Res<Time<Virtual>>,
    mut overlays: Query<&mut Node, With<Overlay>>,
    mut labels: Query<&mut Text, With<ButtonLabel>>,
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<PauseButton>>,
) {
    let paused = time.is_paused();
    for mut node in &mut overlays {
        let display = if paused { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
    for mut text in &mut labels {
        let label = if paused { "RESUME [P]" } else { "PAUSE [P]" };
        if text.0 != label {
            text.0 = label.into();
        }
    }
    for (interaction, mut background) in &mut buttons {
        let color = match interaction {
            Interaction::Pressed => Color::srgb(0.3, 0.25, 0.05),
            Interaction::Hovered => Color::srgb(0.16, 0.16, 0.22),
            Interaction::None => Color::srgb(0.06, 0.07, 0.12),
        };
        if background.0 != color {
            background.0 = color;
        }
    }
}

#[cfg(test)]
#[path = "tests/pause.rs"]
mod tests;
