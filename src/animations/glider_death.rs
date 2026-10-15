//! Glider death after enemy shots, Fuseball contact, or spike collisions.
//! Draws starburst fragments and retreats the camera before level retry.

use crate::animations::RETREAT_END;
use crate::camera::CAMERA_STEP;
use crate::cry;
use crate::game::Game;
use crate::game::Status;
use crate::glider::Glider;
use crate::level::Level;
use crate::mesh::colored_mesh;
use crate::web::WORLD_UNIT;
use crate::web::Web;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use std::f32::consts::TAU;

#[derive(Clone, Copy)]
pub(crate) struct Death {
    pub(crate) tick: u16,
    pub(crate) visible: bool,
    pub(crate) camera_z: f32,
}

impl Death {
    pub(crate) fn retreat(camera_z: f32) -> Self {
        Self {
            tick: 121,
            visible: false,
            camera_z,
        }
    }
    pub(crate) fn new(position: Vec3, web: &Web) -> Self {
        Self {
            tick: 0,
            visible: true,
            camera_z: web.camera_position(position).z,
        }
    }

    pub(crate) fn advance(&mut self) -> bool {
        if self.visible {
            self.tick += 1;
            self.visible = self.tick <= 120;
            self.camera_z += 17.0 / 16.0 * WORLD_UNIT;
            // zap_player calls vp_xform, which moves z back toward its target.
            if self.visible {
                self.camera_z -= 0.5 * WORLD_UNIT;
            }
        } else {
            self.camera_z += 8.0 * WORLD_UNIT;
        }
        self.camera_z >= RETREAT_END
    }
}

#[derive(Component)]
pub(crate) struct ShotBurst(pub(crate) usize);

pub(crate) fn setup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let mesh = meshes.add(zap_mesh());
    for index in 0..4 {
        commands.spawn((
            ShotBurst(index),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: cry::color(255 - index as u8 * 17, 255),
                unlit: true,
                cull_mode: None,
                ..default()
            })),
            RenderLayers::layer(1),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
}

fn zap_mesh() -> Mesh {
    let points = [
        (3., 3.),
        (8., 7.),
        (14., 3.),
        (12., 8.),
        (16., 12.),
        (11., 11.),
        (10., 16.),
        (7., 11.),
        (3., 13.),
        (6., 8.),
    ]
    .map(|(x, y)| Vec3::new((x - 9.0) * 0.075, (y - 9.0) * 0.075, 0.0));
    // Intersect adjacent stroke edges so every peak has a single pointed join.
    let corners: [_; 10] = std::array::from_fn(|index| {
        let p = points[index];
        let incoming = (p - points[(index + points.len() - 1) % points.len()]).normalize();
        let outgoing = (points[(index + 1) % points.len()] - p).normalize();
        let normal = outgoing.cross(Vec3::Z);
        let miter = (incoming.cross(Vec3::Z) + normal).normalize();
        let offset = miter * (0.012 / miter.dot(normal));
        [p - offset, p + offset]
    });
    colored_mesh((0..points.len()).flat_map(|index| {
        let (a, b) = (corners[index], corners[(index + 1) % points.len()]);
        let quad = [a[0], a[1], b[1], b[0]].map(|point| (point, Color::WHITE));
        [[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]]
    }))
}

pub(crate) fn animate(
    level: Res<Level>,
    mut game: ResMut<Game>,
    mut player: Single<&mut Transform, With<Glider>>,
    mut cameras: Query<&mut Transform, (crate::messages::geometry::GameCameras, Without<Glider>)>,
) {
    let Some(mut death) = game.death else {
        return;
    };
    let was_visible = death.visible;
    if death.advance() {
        game.death = None;
        game.status = Status::Respawning;
        for mut camera in &mut cameras {
            camera.translation = level
                .web
                .camera_position(level.web.glider_transform(level.web.start()).translation);
        }
        return;
    }
    if was_visible {
        player.translation.z += WORLD_UNIT / 4.0;
    }
    for mut camera in &mut cameras {
        if death.visible {
            let delta = (player.translation - camera.translation)
                .clamp(Vec3::splat(-CAMERA_STEP), Vec3::splat(CAMERA_STEP));
            camera.translation.x += delta.x;
            camera.translation.y += delta.y;
        }
        camera.translation.z = death.camera_z;
    }
    game.death = Some(death);
}

pub(crate) fn draw(
    game: Res<Game>,
    player: Single<&Transform, With<Glider>>,
    mut bursts: Query<(&ShotBurst, &mut Transform, &mut Visibility), Without<Glider>>,
) {
    for (burst, mut transform, mut visibility) in &mut bursts {
        *visibility = if game.death.is_some_and(|death| death.visible) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if let Some(death) = game.death {
            *transform = **player;
            transform.translation.z += burst.0 as f32 * f32::from(death.tick) * WORLD_UNIT / 2.0;
            transform.rotation *=
                Quat::from_rotation_z(f32::from((death.tick * 12) as u8) * TAU / 256.0);
        }
    }
}

#[cfg(test)]
#[path = "tests/glider_death.rs"]
mod tests;
