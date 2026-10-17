//! Level entry tumbles the approaching web into place while wireframe claw
//! copies contract onto the glider. Advances the approach and draws its trails.

use crate::cry;
use crate::game::Game;
use crate::game::Status;
use crate::glider::Glider;
use crate::level::Level;
use crate::mesh::colored_mesh;
use crate::powerups;
use crate::scene::LaneHighlight;
use crate::web::LANE_UNIT;
use crate::web::WORLD_UNIT;
use crate::web::Web;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use std::f32::consts::TAU;

pub(crate) const APPROACH_TICKS: u16 = 256;

pub(crate) const REZ_TICKS: u16 = 64;

#[derive(Component)]
pub(crate) struct WebRoot;

#[derive(Component)]
pub(crate) struct ClawTrail(pub(crate) usize);

#[derive(Resource)]
pub(crate) struct EntryAssets {
    pub(crate) frames: [Handle<Mesh>; 8],
    pub(crate) surface: Handle<StandardMaterial>,
}

pub(crate) fn setup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    web: &Web,
) -> (Entity, Handle<StandardMaterial>) {
    let frames = std::array::from_fn(|frame| meshes.add(wire_claw_mesh(frame)));
    for index in 0..4 {
        commands.spawn((
            ClawTrail(index),
            Mesh3d(frames[0].clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: cry::color(255 - index as u8, 255),
                unlit: true,
                cull_mode: None,
                ..default()
            })),
            RenderLayers::layer(1),
            Visibility::Hidden,
            Transform::default(),
        ));
    }
    let surface = materials.add(StandardMaterial {
        base_color: Color::WHITE.with_alpha(0.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    commands.insert_resource(EntryAssets {
        frames,
        surface: surface.clone(),
    });
    (
        commands
            .spawn((WebRoot, web_transform(0, web), Visibility::Visible))
            .id(),
        surface,
    )
}

fn web_transform(tick: u16, web: &Web) -> Transform {
    if tick >= APPROACH_TICKS {
        return Transform::default();
    }
    let unit = TAU / 256.0;
    let rotation = if tick == 0 {
        Quat::IDENTITY
    } else {
        Quat::from_rotation_x((128 + tick / 2) as f32 * unit)
            * Quat::from_rotation_y(tick as f32 * unit)
    };
    let center = Vec3::NEG_Z * (web.world_depth() * 0.5);
    Transform::from_translation(
        center - rotation * center - Vec3::Z * f32::from(APPROACH_TICKS - tick) * WORLD_UNIT,
    )
    .with_rotation(rotation)
}

fn vector_pose(progress: i32) -> (usize, f32) {
    let phase = (-(progress + LANE_UNIT / 2)).rem_euclid(LANE_UNIT);
    let frame = ((phase >> 12) & 7) as usize;
    if phase & 0x8000 == 0 {
        (7 - frame, -1.0)
    } else {
        (frame, 1.0)
    }
}

fn wire_claw_mesh(frame: usize) -> Mesh {
    // claw1 through claw8, including their closed eight-edge connection list.
    let vertices = [
        [
            (6., 4.),
            (3., 9.),
            (9., 12.),
            (15., 9.),
            (12., 4.),
            (17., 9.),
            (9., 15.),
            (1., 9.),
        ],
        [
            (5., 4.),
            (3., 10.),
            (10., 12.),
            (15., 9.),
            (12., 4.),
            (17., 9.),
            (10., 15.),
            (1., 10.),
        ],
        [
            (4., 5.),
            (4., 11.),
            (11., 12.),
            (15., 8.),
            (12., 4.),
            (17., 8.),
            (11., 15.),
            (2., 12.),
        ],
        [
            (4., 6.),
            (5., 12.),
            (11., 13.),
            (15., 8.),
            (11., 3.),
            (17., 7.),
            (12., 16.),
            (3., 13.),
        ],
        [
            (3., 8.),
            (6., 13.),
            (12., 13.),
            (15., 7.),
            (11., 3.),
            (17., 6.),
            (13., 16.),
            (4., 14.),
        ],
        [
            (3., 9.),
            (7., 13.),
            (13., 13.),
            (15., 6.),
            (10., 3.),
            (17., 5.),
            (15., 16.),
            (6., 15.),
        ],
        [
            (4., 11.),
            (10., 14.),
            (14., 13.),
            (15., 6.),
            (10., 3.),
            (17., 4.),
            (16., 15.),
            (10., 16.),
        ],
        [
            (5., 14.),
            (11., 14.),
            (14., 12.),
            (15., 6.),
            (9., 3.),
            (16., 4.),
            (17., 13.),
            (11., 16.),
        ],
    ][frame]
        .map(|(x, y)| Vec3::new((x - 9.0) * 0.075, (y - 9.0) * 0.075, 0.0));
    colored_mesh((0..8).flat_map(|index| {
        let (a, b) = (vertices[index], vertices[(index + 1) % 8]);
        let side = (b - a).normalize().cross(Vec3::Z) * 0.012;
        let quad = [a - side, a + side, b + side, b - side].map(|p| (p, Color::WHITE));
        [[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]]
    }))
}

pub(crate) fn advance(mut game: ResMut<Game>) {
    let Some(tick) = game.entry else {
        return;
    };
    if !matches!(game.status, Status::Entering | Status::Playing) {
        return;
    }
    let tick = tick + 1;
    if tick == APPROACH_TICKS {
        game.status = Status::Playing;
    }
    game.entry = (tick < APPROACH_TICKS + REZ_TICKS).then_some(tick);
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn draw(
    level: Res<Level>,
    game: Res<Game>,
    assets: Res<EntryAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut web: Single<(&mut Transform, &mut Visibility), With<WebRoot>>,
    mut player: Single<(&Glider, &Transform, &mut Visibility), Without<WebRoot>>,
    mut trails: Query<
        (&ClawTrail, &mut Transform, &mut Mesh3d, &mut Visibility),
        (Without<Glider>, Without<WebRoot>),
    >,
    mut highlights: Query<
        &mut Visibility,
        (
            With<LaneHighlight>,
            Without<Glider>,
            Without<ClawTrail>,
            Without<WebRoot>,
        ),
    >,
    mut droids: Query<
        &mut Visibility,
        (
            With<powerups::Droid>,
            Without<Glider>,
            Without<ClawTrail>,
            Without<WebRoot>,
            Without<LaneHighlight>,
        ),
    >,
) {
    let tick = game.entry.unwrap_or(APPROACH_TICKS + REZ_TICKS);
    if game.status == Status::Bonus {
        web.1.set_if_neq(Visibility::Hidden);
        player.2.set_if_neq(Visibility::Hidden);
        for (_, _, _, mut visibility) in &mut trails {
            visibility.set_if_neq(Visibility::Hidden);
        }
        for mut visibility in &mut highlights {
            visibility.set_if_neq(Visibility::Hidden);
        }
        for mut visibility in &mut droids {
            visibility.set_if_neq(Visibility::Hidden);
        }
        return;
    }
    let (web_transform, web_visibility) = &mut *web;
    web_transform.set_if_neq(self::web_transform(tick, &level.web));
    web_visibility.set_if_neq(
        if game
            .exit
            .is_some_and(|exit| !exit.web_visible || exit.flash > 0)
        {
            Visibility::Hidden
        } else {
            Visibility::Visible
        },
    );
    let alpha = game.exit.map_or_else(
        || (f32::from(tick.saturating_sub(APPROACH_TICKS)) * 8.0 / 248.0).min(1.0),
        |exit| (1.0 - f32::from(exit.ticks) * 8.0 / 248.0).max(0.0),
    );
    crate::mesh::set_base_color(
        &mut materials,
        &assets.surface,
        Color::WHITE.with_alpha(game.death.map_or(alpha, |death| {
            (1.0 - f32::from(death.tick) * 8.0 / 248.0).max(0.0)
        })),
    );
    let (glider, transform, visibility) = &mut *player;
    if game.entry.is_some()
        || game.exit.is_some_and(|exit| exit.wireframe)
        || (game.death.is_some() && game.status != Status::Spiked)
    {
        visibility.set_if_neq(Visibility::Hidden);
    }
    // check_game owns the invulnerability blink once the claw has rezzed.
    else if game.invulnerable == 0 {
        visibility.set_if_neq(Visibility::Visible);
    }
    for mut visibility in &mut highlights {
        visibility.set_if_neq(
            if tick < APPROACH_TICKS
                || game.exit.is_some_and(|exit| exit.wireframe)
                || game.death.is_some()
            {
                Visibility::Hidden
            } else {
                Visibility::Visible
            },
        );
    }
    for mut visibility in &mut droids {
        visibility.set_if_neq(
            if tick < APPROACH_TICKS || game.exit.is_some() || game.death.is_some() {
                Visibility::Hidden
            } else {
                Visibility::Visible
            },
        );
    }
    let (frame, mirror) = vector_pose(glider.progress);
    for (trail, mut copy, mut mesh, mut visibility) in &mut trails {
        let exiting = game
            .exit
            .is_some_and(|exit| exit.wireframe && exit.flash == 0);
        visibility.set_if_neq(
            if game.death.is_none()
                && (exiting || (APPROACH_TICKS + 1..APPROACH_TICKS + REZ_TICKS).contains(&tick))
            {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
        );
        let mut target = **transform;
        let spacing = game
            .exit
            .map_or(f32::from(APPROACH_TICKS + REZ_TICKS - tick) / 4.0, |exit| {
                exit.spacing as f32 / LANE_UNIT as f32
            });
        target.translation.z += trail.0 as f32 * spacing * WORLD_UNIT;
        if let Some(exit) = game.exit {
            target.rotation *= Quat::from_rotation_z(f32::from(exit.spin) * TAU / 256.0);
        }
        target.scale.x = mirror;
        copy.set_if_neq(target);
        if mesh.0 != assets.frames[frame] {
            mesh.0 = assets.frames[frame].clone();
        }
    }
}

#[cfg(test)]
#[path = "tests/level_entry.rs"]
mod tests;
