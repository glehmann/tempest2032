//! Original score digits, claw lives, and warp pyramids drawn as vector meshes.

use crate::game::Game;
use crate::messages::{message_mesh, shaded_graphic};
use bevy::camera::visibility::RenderLayers;
use bevy::diagnostic::{
    Diagnostic, DiagnosticPath, DiagnosticsStore, SystemInformationDiagnosticsPlugin,
};
use bevy::prelude::*;
use bevy::sprite_render::AlphaMode2d;
use std::collections::VecDeque;
use std::time::Duration;

const FPS_REFRESH_INTERVAL: Duration = Duration::from_millis(500);
const FRAME_WINDOW: Duration = Duration::from_secs(2);

#[derive(Resource)]
pub(crate) struct HudAssets {
    digits: [Vec<[(Vec2, Color); 3]>; 10],
    life: Vec<[(Vec2, Color); 3]>,
    mesh: Handle<Mesh>,
    previous: Option<(u32, u8, u8, f32, bool)>,
}

pub(crate) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let bytes = include_bytes!("../assets/messages/hud.bin");
    assert_eq!(&bytes[..4], b"T2KH");
    let mut data = &bytes[4..];
    let digits = std::array::from_fn(|_| shaded_graphic(&mut data));
    let life = shaded_graphic(&mut data);
    assert!(data.is_empty());
    let mesh = meshes.add(message_mesh(Vec::new()));
    commands.spawn((
        Mesh2d(mesh.clone()),
        MeshMaterial2d(materials.add(ColorMaterial {
            alpha_mode: AlphaMode2d::Blend,
            ..default()
        })),
        RenderLayers::layer(2),
        Transform::from_xyz(0., 0., 1.),
        bevy::camera::visibility::NoFrustumCulling,
    ));
    commands.insert_resource(HudAssets {
        digits,
        life,
        mesh,
        previous: None,
    });
}

#[derive(Component)]
pub(crate) struct FpsLabel;

/// Counts frames between readings and retains frame times over a rolling window.
#[derive(Resource)]
pub(crate) struct FpsMeter {
    frames: u32,
    elapsed: Duration,
    refresh: Timer,
    frame_times: VecDeque<(Duration, f64)>,
}

impl Default for FpsMeter {
    fn default() -> Self {
        Self {
            frames: 0,
            elapsed: Duration::ZERO,
            refresh: Timer::new(FPS_REFRESH_INTERVAL, TimerMode::Repeating),
            frame_times: VecDeque::new(),
        }
    }
}

pub(crate) fn update_hud(
    game: Res<Game>,
    upgrades: Res<crate::powerups::Upgrades>,
    time: Res<Time<Virtual>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut assets: ResMut<HudAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let width = windows
        .iter()
        .next()
        .map_or(384., |w| 240. * w.width() / w.height().max(1.));
    let state = (
        game.score,
        game.lives,
        upgrades.warp_tokens(),
        width,
        time.is_paused(),
    );
    if assets.previous == Some(state) {
        return;
    }
    let mut triangles = Vec::new();
    let point = |p: Vec2| Vec3::new(p.x - width / 2., 120. - p.y, 0.);
    // ashowscore/setlives use x=48, y=10/30, with 16/32-unit advances.
    let fit = (width / 384.).min(1.);
    let left = 48. * fit;
    for (i, digit) in game.score.to_string().bytes().enumerate() {
        triangles.extend(
            assets.digits[usize::from(digit - b'0')]
                .iter()
                .map(|triangle| {
                    triangle.map(|(p, color)| {
                        (
                            point(p * fit + Vec2::new(left + i as f32 * 16. * fit, 10.)),
                            color,
                        )
                    })
                }),
        );
    }
    for life in 0..game.lives.min(8) {
        triangles.extend(assets.life.iter().map(|triangle| {
            triangle.map(|(p, color)| {
                (
                    point(p * fit + Vec2::new(left + f32::from(life) * 32. * fit, 30.)),
                    color,
                )
            })
        }));
    }
    // makepyr: 31x20, centre 352/320/288, base y=27, tip y=7, hub y=19.
    for token in 0..upgrades.warp_tokens().min(3) {
        let center = width - (32. + f32::from(token) * 32.) * fit;
        let vertices = [
            Vec2::new(-15., 20.),
            Vec2::ZERO,
            Vec2::new(16., 20.),
            Vec2::new(0., 12.),
        ];
        let colors = [
            crate::cry::color(0x8f, 95),
            crate::cry::color(0x8f, 95),
            crate::cry::color(0x8f, 95),
            crate::cry::color(0x88, 255),
        ];
        for face in [[0, 1, 3], [1, 2, 3], [2, 0, 3]] {
            triangles
                .push(face.map(|i| (point(vertices[i] * fit + Vec2::new(center, 7.)), colors[i])));
        }
    }
    let gold = Color::srgb(1., 0.85, 0.15);
    if time.is_paused() {
        triangles.push(
            [
                Vec2::new(-10., -14.),
                Vec2::new(14., 0.),
                Vec2::new(-10., 14.),
            ]
            .map(|p| (p.extend(2.), gold)),
        );
    }
    *meshes.get_mut(&assets.mesh).unwrap() = message_mesh(triangles);
    assets.previous = Some(state);
}

pub(crate) fn spawn_fps_label(mut commands: Commands) {
    commands.spawn((
        FpsLabel,
        Text::default(),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.4, 0.4, 0.5)),
        Node {
            position_type: PositionType::Absolute,
            top: px(48),
            right: px(26),
            ..default()
        },
    ));
}

pub(crate) fn update_fps(
    time: Res<Time<Real>>,
    diagnostics: Res<DiagnosticsStore>,
    mut meter: ResMut<FpsMeter>,
    mut label: Single<&mut Text, With<FpsLabel>>,
) {
    let delta = time.delta();
    if delta.is_zero() {
        return;
    }
    let now = time.elapsed();
    meter
        .frame_times
        .push_back((now, delta.as_secs_f64() * 1000.0));
    while meter
        .frame_times
        .front()
        .is_some_and(|(at, _)| now - *at > FRAME_WINDOW)
    {
        meter.frame_times.pop_front();
    }
    meter.frames += 1;
    meter.elapsed += delta;
    if !meter.refresh.tick(delta).just_finished() {
        return;
    }
    let fps = meter.frames as f64 / meter.elapsed.as_secs_f64();
    meter.frames = 0;
    meter.elapsed = Duration::ZERO;
    let (fastest, slowest) = meter
        .frame_times
        .iter()
        .fold((f64::MAX, 0.0_f64), |(fastest, slowest), (_, ms)| {
            (fastest.min(*ms), slowest.max(*ms))
        });
    let cpu = average(
        &diagnostics,
        &SystemInformationDiagnosticsPlugin::PROCESS_CPU_USAGE,
    )
    .map_or(String::new(), |usage| format!("  CPU {usage:.0}%"));
    let memory = average(
        &diagnostics,
        &SystemInformationDiagnosticsPlugin::PROCESS_MEM_USAGE,
    )
    .map_or(String::new(), |gib| {
        format!("  MEM {:.0} MiB", gib * 1024.0)
    });
    let text = format!("{fps:.0} FPS  {fastest:.1}-{slowest:.1} ms{cpu}{memory}");
    if label.0 != text {
        label.0 = text;
    }
}

fn average(diagnostics: &DiagnosticsStore, path: &DiagnosticPath) -> Option<f64> {
    diagnostics.get(path).and_then(Diagnostic::average)
}

#[cfg(test)]
#[path = "tests/hud.rs"]
mod tests;
