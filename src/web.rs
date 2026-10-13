//! Web topology and geometry. A web is the lane-lined tunnel on which the glider
//! and enemies move; progress uses 16:16 fixed-point lane coordinates.

use bevy::prelude::*;
use serde::Deserialize;

pub(crate) const FRONT_RADIUS: f32 = 4.0;

pub(crate) const BACK_RADIUS: f32 = FRONT_RADIUS;

pub(crate) const WORLD_UNIT: f32 = FRONT_RADIUS / 24.0;

pub(crate) const LANE_UNIT: i32 = 1 << 16;

fn closed_web() -> bool {
    true
}

#[derive(Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Web {
    #[serde(default = "closed_web")]
    pub(crate) closed: bool,
    pub(crate) vertices: Vec<[f32; 2]>,
    pub(crate) orientations: Vec<i16>,
    pub(crate) start_lane: usize,
    pub(crate) depth: u16,
}

impl Web {
    pub(crate) fn lanes(&self) -> usize {
        self.vertices
            .len()
            .saturating_sub(usize::from(!self.closed))
    }
    pub(crate) fn period(&self) -> i32 {
        self.lanes() as i32 * LANE_UNIT
    }
    pub(crate) fn start(&self) -> i32 {
        self.start_lane as i32 * LANE_UNIT
    }
    pub(crate) fn rear(&self) -> i32 {
        i32::from(self.depth) * LANE_UNIT
    }
    pub(crate) fn world_depth(&self) -> f32 {
        f32::from(self.depth) * WORLD_UNIT
    }
    pub(crate) fn selected_lane(&self, progress: i32) -> f32 {
        if self.closed {
            ((progress + LANE_UNIT / 2).rem_euclid(self.period()) / LANE_UNIT) as f32
        } else {
            ((progress + LANE_UNIT / 2) / LANE_UNIT).clamp(0, self.lanes() as i32 - 1) as f32
        }
    }
    pub(crate) fn normalize(&self, progress: i32) -> i32 {
        if self.closed {
            progress.rem_euclid(self.period())
        } else {
            progress.clamp(0, (self.lanes() as i32 - 1) * LANE_UNIT)
        }
    }
    pub(crate) fn neighbor(&self, lane: usize, direction: i32) -> Option<usize> {
        let next = lane as i32 + direction;
        if self.closed {
            Some(next.rem_euclid(self.lanes() as i32) as usize)
        } else {
            (0..self.lanes() as i32)
                .contains(&next)
                .then_some(next as usize)
        }
    }
    pub(crate) fn lane_delta(&self, from: i32, to: i32) -> i32 {
        if self.closed {
            (to - from + self.period() / 2).rem_euclid(self.period()) - self.period() / 2
        } else {
            to - from
        }
    }
    pub(crate) fn vertex(&self, index: f32, radius: f32, z: f32) -> Vec3 {
        let index = if self.closed {
            index.rem_euclid(self.lanes() as f32)
        } else {
            index.clamp(0.0, self.lanes() as f32)
        };
        let [x, y] = self.vertices[index as usize];
        Vec3::new((x - 8.0) * radius / 6.0, (8.0 - y) * radius / 6.0, z)
    }
    pub(crate) fn glider_transform(&self, progress: i32) -> Transform {
        let lane = self.selected_lane(progress);
        let a = self.vertex(lane, FRONT_RADIUS, 0.0);
        let b = self.vertex(lane + 1.0, FRONT_RADIUS, 0.0);
        let tangent = b - a;
        Transform::from_translation(a.lerp(b, 0.5) + Vec3::Z * 0.12)
            .with_rotation(Quat::from_rotation_z(tangent.y.atan2(tangent.x)))
    }
    pub(crate) fn camera_position(&self, glider: Vec3) -> Vec3 {
        let initial = self.glider_transform(self.start()).translation;
        if self.horizontal_plane() {
            return Vec3::new(
                glider.x * 0.5,
                glider.y * 0.5,
                30. * WORLD_UNIT + glider.z - initial.z,
            );
        }
        Vec3::new(-0.3 + initial.x * 0.15, -0.95 + initial.y * 0.15, 5.3)
            + (glider - initial) * Vec3::new(0.5, 0.5, 1.0)
    }
    pub(crate) fn horizontal_plane(&self) -> bool {
        !self.closed
            && self
                .vertices
                .first()
                .is_some_and(|first| self.vertices.iter().all(|vertex| vertex[1] == first[1]))
    }
    pub(crate) fn camera_rotation(&self) -> Quat {
        Quat::from_euler(
            EulerRot::YXZ,
            if self.horizontal_plane() { 0. } else { 0.105 },
            0.06,
            0.,
        )
    }
    pub(crate) fn orientation(&self, lane: usize) -> Quat {
        Quat::from_rotation_z(f32::from(self.orientations[lane]) * std::f32::consts::TAU / 256.0)
    }
    pub(crate) fn flip_angle(&self, from: usize, to: usize, direction: i32) -> u16 {
        let turn = (direction * i32::from(self.orientations[from] - self.orientations[to]))
            .rem_euclid(256);
        (128 - turn).rem_euclid(256) as u16
    }
}

#[cfg(test)]
#[path = "tests/web.rs"]
mod tests;
