//! Beveled 3D score digits, actual claw models, and original warp pyramids.

use crate::game::Game;
use crate::messages::geometry::{self, Glyphs, Style, Triangle};
use bevy::camera::visibility::RenderLayers;
#[cfg(not(target_arch = "wasm32"))]
use bevy::diagnostic::{
    Diagnostic, DiagnosticPath, DiagnosticsStore, SystemInformationDiagnosticsPlugin,
};
use bevy::prelude::*;
use std::collections::VecDeque;
use std::time::Duration;

const FPS_REFRESH_INTERVAL: Duration = Duration::from_millis(500);
const FRAME_WINDOW: Duration = Duration::from_secs(2);

#[derive(Resource)]
pub(crate) struct HudAssets {
    life: Vec<Triangle>,
    mesh: Handle<Mesh>,
    previous: Option<(u32, u16, u8, f32)>,
    icons: [f32; 11],
}

pub(crate) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let life = crate::glider::claw_triangles(4)
        .into_iter()
        .map(|t| {
            t.map(|(p, c)| {
                let scale = 26. / 1.2;
                (
                    Vec3::new((p.x + 0.6) * scale, (p.y + 0.375) * scale, p.z * scale),
                    c,
                )
            })
        })
        .collect();
    let mesh = meshes.add(geometry::mesh(Vec::new()));
    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            unlit: true,
            cull_mode: None,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        RenderLayers::layer(3),
        bevy::camera::visibility::NoFrustumCulling,
    ));
    commands.insert_resource(HudAssets {
        life,
        mesh,
        previous: None,
        icons: [0.; 11],
    });
}

#[derive(Component)]
pub(crate) struct FpsLabel;

#[derive(Resource)]
pub(crate) struct FpsDisplay(pub(crate) bool);

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
    time: Res<Time<Virtual>>,
    game: Res<Game>,
    upgrades: Res<crate::powerups::Upgrades>,
    glyphs: Res<Glyphs>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut assets: ResMut<HudAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let width = windows
        .iter()
        .next()
        .map_or(384., |w| 240. * w.width() / w.height().max(1.));
    let state = (game.score, game.lives, upgrades.warp_tokens(), width);
    let mut animating = false;
    let initialized = assets.previous.is_some();
    for (i, progress) in assets.icons.iter_mut().enumerate() {
        let visible = if i < 8 {
            i < usize::from(game.lives.saturating_sub(1).min(8))
        } else {
            i - 8 < usize::from(upgrades.warp_tokens().min(3))
        };
        let target = if visible { 1. } else { 0. };
        let previous = *progress;
        *progress = if initialized {
            let step = time.delta_secs() / 0.25;
            *progress + (target - *progress).clamp(-step, step)
        } else {
            target
        };
        animating |= *progress != previous;
    }
    if assets.previous == Some(state) && !animating {
        return;
    }
    let mut triangles = Vec::new();
    let point = |p: Vec2| Vec3::new(p.x - width / 2., 120. - p.y, 20.);
    // Match the warp pyramids' 16-unit outer margin; retain y=10/30 and 16/32-unit advances.
    let fit = (width / 384.).min(1.);
    let left = 16. * fit;
    glyphs.place(
        &mut triangles,
        &game.score.to_string(),
        Style::Rainbow,
        point(Vec2::new(left, 10.)),
        Vec3::splat(fit),
        16. * fit,
    );
    for (life, &progress) in assets.icons[..8].iter().enumerate() {
        if progress == 0. {
            continue;
        }
        let fade = progress * progress * (3. - 2. * progress);
        let scale = 0.75 + 0.25 * fade;
        triangles.extend(assets.life.iter().map(|triangle| {
            triangle.map(|(p, color)| {
                let p = (p - Vec3::new(13., 8., 0.)) * scale + Vec3::new(13., 8., 0.);
                (
                    point(p.truncate() * fit + Vec2::new(left + life as f32 * 32. * fit, 30.))
                        + Vec3::Z * p.z * fit,
                    color.with_alpha(fade),
                )
            })
        }));
    }
    // makepyr: 31x20, centre 352/320/288, base y=27, tip y=7, hub y=19.
    for (token, &progress) in assets.icons[8..].iter().enumerate() {
        if progress == 0. {
            continue;
        }
        let fade = progress * progress * (3. - 2. * progress);
        let scale = 0.75 + 0.25 * fade;
        let center = width - (32. + token as f32 * 32.) * fit;
        let vertices = [
            Vec2::new(-15., 20.),
            Vec2::ZERO,
            Vec2::new(16., 20.),
            Vec2::new(0., 12.),
        ];
        let colors = [
            Color::srgb_u8(0, 95, 0),
            Color::srgb_u8(95, 0, 0),
            Color::srgb_u8(0, 95, 0),
            Color::WHITE,
        ];
        for face in [[0, 1, 3], [1, 2, 3], [2, 0, 3]] {
            triangles.push(face.map(|i| {
                let p = (vertices[i] - Vec2::new(0., 10.)) * scale + Vec2::new(0., 10.);
                (
                    point(p * fit + Vec2::new(center, 7.)),
                    colors[i].with_alpha(fade),
                )
            }));
        }
    }
    // Blended meshes do not write depth, so draw rear faces before front faces.
    triangles.sort_by(|a, b| {
        a.iter()
            .map(|(p, _)| p.z)
            .sum::<f32>()
            .total_cmp(&b.iter().map(|(p, _)| p.z).sum::<f32>())
    });
    *meshes.get_mut(&assets.mesh).unwrap() = geometry::mesh(triangles);
    assets.previous = Some(state);
}

pub(crate) fn spawn_fps_label(mut commands: Commands, display: Res<FpsDisplay>) {
    commands.spawn((
        FpsLabel,
        if display.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
        GlobalZIndex(20),
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

pub(crate) fn toggle_fps(
    keys: Res<ButtonInput<KeyCode>>,
    mut display: ResMut<FpsDisplay>,
    mut meter: ResMut<FpsMeter>,
    mut label: Single<&mut Visibility, With<FpsLabel>>,
) {
    if keys.just_pressed(KeyCode::F3) {
        display.0 = !display.0;
        // The meter pauses while hidden, so start a fresh reading.
        *meter = default();
        **label = if display.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub(crate) fn update_fps(
    time: Res<Time<Real>>,
    antialiasing: Res<crate::antialiasing::Antialiasing>,
    #[cfg(not(target_arch = "wasm32"))] diagnostics: Res<DiagnosticsStore>,
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
    #[cfg(not(target_arch = "wasm32"))]
    let cpu = average(
        &diagnostics,
        &SystemInformationDiagnosticsPlugin::PROCESS_CPU_USAGE,
    )
    .map_or(String::new(), |usage| format!("  CPU {usage:.0}%"));
    #[cfg(not(target_arch = "wasm32"))]
    let memory = average(
        &diagnostics,
        &SystemInformationDiagnosticsPlugin::PROCESS_MEM_USAGE,
    )
    .map_or(String::new(), |gib| {
        format!("  MEM {:.0} MiB", gib * 1024.0)
    });
    let text = format!(
        "{fps:.0} FPS  {fastest:.1}-{slowest:.1} ms  AA {}",
        antialiasing.quality.label()
    );
    #[cfg(not(target_arch = "wasm32"))]
    let text = format!("{text}{cpu}{memory}");
    if label.0 != text {
        label.0 = text;
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn average(diagnostics: &DiagnosticsStore, path: &DiagnosticPath) -> Option<f64> {
    diagnostics.get(path).and_then(Diagnostic::average)
}

#[cfg(test)]
#[path = "tests/hud.rs"]
mod tests;
