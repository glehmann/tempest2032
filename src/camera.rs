//! Camera tracking and viewport fitting for the active web and player glider.

use crate::glider::Glider;
use crate::level::Level;
use crate::web::FRONT_RADIUS;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub(crate) const CAMERA_STEP: f32 = FRONT_RADIUS / 96.0;

#[derive(Component)]
pub(crate) struct WebCamera;

#[allow(clippy::type_complexity)]
pub(crate) fn follow_glider(
    level: Res<Level>,
    glider: Single<&Transform, With<Glider>>,
    mut cameras: Query<
        (&mut Transform, Option<&WebCamera>),
        (
            With<Camera3d>,
            Without<Glider>,
            Without<crate::messages::geometry::OverlayCamera>,
        ),
    >,
) {
    for (mut transform, managed) in &mut cameras {
        if managed.is_some() {
            transform.rotation = level.web.camera_rotation();
        }
        let target = level.web.camera_position(glider.translation);
        let delta = target - transform.translation;
        transform.translation += delta.clamp(Vec3::splat(-CAMERA_STEP), Vec3::splat(CAMERA_STEP));
        transform.translation.z = target.z;
    }
}

pub(crate) fn fit_camera(
    level: Res<Level>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut cameras: Query<
        &mut Projection,
        (
            With<Camera3d>,
            Without<crate::messages::geometry::OverlayCamera>,
        ),
    >,
) {
    let aspect = window.width() / window.height().max(1.0);
    let fov = if level.web.horizontal_plane() {
        // llama.gas projects with a 128-pixel focal length on a 384×240 canvas.
        2. * (120. / 128. * (1.6 / aspect).max(1.)).atan()
    } else {
        let [width, height] =
            level
                .web
                .vertices
                .iter()
                .fold([4.0_f32, 4.0_f32], |[w, h], [x, y]| {
                    [
                        w.max(((x - 8.0) * FRONT_RADIUS / 6.0).abs()),
                        h.max(((y - 8.0) * FRONT_RADIUS / 6.0).abs()),
                    ]
                });
        let half_height = (5.0 * (1.0_f32.max(1.294 / aspect)))
            .max(height + 1.0)
            .max((width + 1.0) / aspect);
        2.0 * (half_height / 5.3).atan()
    };
    for mut projection in &mut cameras {
        if let Projection::Perspective(perspective) = projection.bypass_change_detection()
            && perspective.fov != fov
        {
            perspective.fov = fov;
            projection.set_changed();
        }
    }
}

#[cfg(test)]
#[path = "tests/camera.rs"]
mod tests;
