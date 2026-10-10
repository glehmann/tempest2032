//! Manual full sweep and last-chance zap, with yak.s fire/run_objects cadence.

use super::Upgrades;
use crate::{
    enemies::{
        Enemy, SpawnOrder,
        aliens::AlienDroid,
        beasts::Reflected,
        flipper_render::FlipperAssets,
        flippers::{Flipper, Motion},
        fuseballs::{self, Fuseball},
        mirrors::ReturnedShot,
        pulsar_render::PulsarAssets,
        pulsars::Pulsar,
        shots::EnemyShot,
        spikers::Spiker,
        tankers::{self, Contents, Tanker, TankerAssets},
    },
    explosion,
    game::{Game, Status},
    glider::Glider,
    messages::{Id, message_mesh},
};
use bevy::{camera::visibility::RenderLayers, prelude::*};

pub(crate) struct Superzapper {
    uses: u8,
    pub(crate) queued: bool,
    active: bool,
    single: bool,
    delay: u8,
    bolt: Option<Vec3>,
    bolt_ticks: u8,
    frame: u32,
}

impl Default for Superzapper {
    fn default() -> Self {
        Self {
            uses: 2,
            queued: false,
            active: false,
            single: false,
            delay: 0,
            bolt: None,
            bolt_ticks: 0,
            frame: 0,
        }
    }
}

impl Superzapper {
    pub(super) fn cancel(&mut self) {
        self.active = false;
        self.queued = false;
        self.bolt = None;
        self.bolt_ticks = 0;
        self.delay = 0;
    }

    pub(super) fn powerup(&mut self) {
        // sprzap restores the desperate zap if both manual uses were spent.
        self.uses = self.uses.max(1);
        self.active = true;
        self.single = false;
        self.delay = 0;
        self.queued = false;
    }
}

pub(crate) fn queue(
    keys: Res<ButtonInput<KeyCode>>,
    game: Res<Game>,
    mut upgrades: ResMut<Upgrades>,
) {
    let zapper = &mut upgrades.zapper;
    if game.status == Status::Playing
        && !keys.just_pressed(KeyCode::KeyR)
        && zapper.uses > 0
        && !zapper.active
    {
        zapper.queued |= keys.just_pressed(KeyCode::KeyZ);
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn advance(
    mut commands: Commands,
    mut game: ResMut<Game>,
    level: Res<crate::level::Level>,
    mut upgrades: ResMut<Upgrades>,
    bursts: Res<explosion::BurstAssets>,
    flippers: Res<FlipperAssets>,
    shells: Res<TankerAssets>,
    fuses: Res<fuseballs::FuseAssets>,
    pulsars: Res<PulsarAssets>,
    targets: Query<
        (
            Entity,
            &Transform,
            Option<&Flipper>,
            Option<&Tanker>,
            Option<&Contents>,
            Option<&Fuseball>,
            Option<&Pulsar>,
            Option<&Spiker>,
            Option<&AlienDroid>,
            Option<&EnemyShot>,
            Option<&Reflected>,
            Option<&ReturnedShot>,
            &SpawnOrder,
        ),
        Or<(With<Enemy>, With<EnemyShot>)>,
    >,
) {
    let zapper = &mut upgrades.zapper;
    if game.status != Status::Playing {
        zapper.cancel();
        return;
    }
    zapper.frame = zapper.frame.wrapping_add(1);
    zapper.bolt_ticks = zapper.bolt_ticks.saturating_sub(1);
    if std::mem::take(&mut zapper.queued) && !zapper.active && zapper.uses > 0 {
        zapper.single = zapper.uses == 1;
        zapper.uses -= 1;
        zapper.active = true;
        zapper.delay = 0;
        if !zapper.single {
            game.announce(Id::EatElectricDeath);
        }
        crate::sound::play(&mut commands, crate::sound::Effect::Crackle);
    }
    if !zapper.active {
        return;
    }
    if zapper.delay > 0 {
        zapper.delay -= 1;
        return;
    }
    // collie bypasses shot windows and Beastie armor. mcollie-only enemies,
    // approaching pixels, spikes, and reflected shots cannot consume a zap.
    let target = targets
        .iter()
        .filter(
            |(
                _,
                _,
                flipper,
                tanker,
                _,
                fuse,
                pulsar,
                spiker,
                alien,
                shot,
                reflected,
                returned,
                _,
            )| {
                if let Some(flipper) = flipper {
                    !matches!(flipper.motion, Motion::Approach)
                } else if let Some(tanker) = tanker {
                    !tanker.approach
                } else if let Some(fuse) = fuse {
                    !matches!(fuse.motion, fuseballs::Motion::Approach)
                } else if let Some(pulsar) = pulsar {
                    !pulsar.approach
                } else if spiker.is_some() {
                    true
                } else if let Some(alien) = alien {
                    !alien.approach && !alien.climbing()
                } else {
                    shot.is_some() && !zapper.single && reflected.is_none() && returned.is_none()
                }
            },
        )
        .max_by_key(|(.., order)| order.0);
    let Some((entity, pose, flipper, tanker, contents, fuse, pulsar, spiker, alien, _, _, _, _)) =
        target
    else {
        zapper.active = false;
        return;
    };
    if let Some(tanker) = tanker {
        tankers::open(
            &mut commands,
            &mut game,
            &level,
            &flippers,
            &shells,
            &fuses,
            &pulsars,
            tanker,
            *contents.unwrap(),
        );
    } else {
        let points = if let Some(pulsar) = pulsar {
            pulsar.points()
        } else if flipper.is_some() || spiker.is_some() {
            150
        } else if fuse.is_some() || alien.is_some() {
            game.random = game.random.wrapping_mul(1664525).wrapping_add(1013904223);
            250 * (1 + ((game.random >> 16) % 3))
        } else {
            0
        };
        game.score += points;
        if fuse.is_some() || alien.is_some() {
            game.score_popup(points, pose.translation);
        }
    }
    commands.entity(entity).despawn();
    explosion::spawn_explosion(&mut commands, &bursts, pose.translation);
    if zapper.single {
        // r_end1 falls through clzapa and clears bolt_lock before drawing.
        zapper.cancel();
    } else {
        zapper.bolt = Some(pose.translation);
        zapper.bolt_ticks = 4;
        zapper.delay = 3;
    }
}

#[derive(Resource)]
pub(crate) struct BoltAssets(Handle<Mesh>);

pub(crate) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mesh = meshes.add(message_mesh(Vec::new()));
    commands.spawn((
        Mesh2d(mesh.clone()),
        MeshMaterial2d(materials.add(ColorMaterial::default())),
        RenderLayers::layer(2),
        bevy::camera::visibility::NoFrustumCulling,
    ));
    commands.insert_resource(BoltAssets(mesh));
}

fn bolt_triangles(start: Vec2, end: Vec2, frame: u32) -> Vec<[(Vec3, Color); 3]> {
    let normal = (end - start).normalize_or_zero().perp();
    let vertices: [Vec2; 33] = std::array::from_fn(|i| {
        let jitter = if i == 0 || i == 32 {
            0.
        } else {
            let noise = (i as u32)
                .wrapping_mul(747796405)
                .wrapping_add(frame.wrapping_mul(2891336453));
            ((noise ^ (noise >> 16)) & 255) as f32 / 255. * 4. - 2.
        };
        start.lerp(end, i as f32 / 32.) + normal * jitter
    });
    let hue = frame.wrapping_mul(16) as u8;
    let mut triangles = Vec::with_capacity(128);
    for (width, color) in [(0.8, crate::cry::color(hue, 150)), (0.2, Color::WHITE)] {
        for edge in vertices.windows(2) {
            let side = (edge[1] - edge[0]).normalize_or_zero().perp() * width;
            let corners = [
                edge[0] - side,
                edge[0] + side,
                edge[1] + side,
                edge[1] - side,
            ];
            for face in [[0, 1, 2], [0, 2, 3]] {
                triangles.push(face.map(|i| (corners[i].extend(0.5), color)));
            }
        }
    }
    triangles
}

pub(crate) fn draw(
    upgrades: Res<Upgrades>,
    game: Res<Game>,
    assets: Res<BoltAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    player: Single<&Transform, With<Glider>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    let zapper = &upgrades.zapper;
    let projected = zapper
        .bolt
        .filter(|_| zapper.bolt_ticks > 0 && game.status == Status::Playing)
        .and_then(|target| {
            cameras.iter().find_map(|(camera, pose)| {
                let size = camera.logical_viewport_size()?;
                let project = |p| {
                    camera
                        .world_to_viewport(pose, p)
                        .ok()
                        .map(|p| Vec2::new(p.x - size.x / 2., size.y / 2. - p.y) * (240. / size.y))
                };
                Some((project(player.translation)?, project(target)?))
            })
        });
    *meshes.get_mut(&assets.0).unwrap() =
        message_mesh(projected.map_or_else(Vec::new, |(start, end)| {
            bolt_triangles(start, end, zapper.frame)
        }));
}

#[cfg(test)]
#[path = "tests/superzapper.rs"]
mod tests;
