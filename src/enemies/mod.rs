//! Enemy actor marker and asset setup for the web enemies. Enemy-specific
//! movement, spawning, and graphics live in the child modules.

use crate::animations::glider_death as shot_death;
use crate::animations::level_exit as exit;
use crate::enemies::flipper_render::FlipperAssets;
use crate::enemies::flipper_render::flipper_dot_color;
use crate::enemies::flipper_render::flipper_mesh;
use crate::enemies::shots::enemy_shot_mesh;
use crate::explosion;
use crate::hud::GameHud;
use crate::level::Level;
use crate::powerups;
use bevy::prelude::*;

pub(crate) mod beasts;
pub(crate) mod flipper_render;
pub(crate) mod flippers;
pub(crate) mod fuseballs;
pub(crate) mod pulsar_render;
pub(crate) mod pulsars;
pub(crate) mod shots;
pub(crate) mod spikers;
pub(crate) mod tankers;

#[derive(Component, Default)]
pub(crate) struct Enemy;

pub(crate) fn setup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    level: &Level,
) {
    explosion::setup(commands, meshes, materials);
    powerups::setup(commands, meshes, materials);
    exit::setup(commands);
    shot_death::setup(commands, meshes, materials);
    tankers::setup(commands, meshes, materials);
    spikers::setup(commands, meshes, materials);
    beasts::setup(commands, meshes);
    fuseballs::setup(commands, meshes, materials);
    pulsar_render::setup(commands, meshes, materials);
    commands.insert_resource(FlipperAssets {
        body: meshes.add(flipper_mesh(level.appearance.flipper)),
        point: meshes.add(Rectangle::new(1.0, 1.0)),
        shot: meshes.add(enemy_shot_mesh()),
        shaded: materials.add(StandardMaterial {
            unlit: true,
            cull_mode: None,
            ..default()
        }),
        point_shades: (0..=255u16)
            .map(|intensity| {
                materials.add(StandardMaterial {
                    base_color: flipper_dot_color(level.appearance.flipper, intensity),
                    unlit: true,
                    cull_mode: None,
                    ..default()
                })
            })
            .collect(),
    });
    commands.spawn((
        GameHud,
        Text::default(),
        TextFont {
            font_size: 22.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.85, 0.25)),
        Node {
            position_type: PositionType::Absolute,
            left: px(26),
            top: px(22),
            ..default()
        },
    ));
}
