use super::*;

#[derive(Clone)]
pub(crate) struct Web {
    pub(crate) vertices: Vec<[f32; 2]>,
    pub(crate) orientations: Vec<i16>,
    pub(crate) start_lane: usize,
    pub(crate) depth: u16,
}

impl Web {
    pub(crate) fn lanes(&self) -> usize {
        self.vertices.len()
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
        ((progress + LANE_UNIT / 2).rem_euclid(self.period()) / LANE_UNIT) as f32
    }
    pub(crate) fn vertex(&self, index: f32, radius: f32, z: f32) -> Vec3 {
        let [x, y] = self.vertices[index.rem_euclid(self.lanes() as f32) as usize];
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
        Vec3::new(-0.3 + initial.x * 0.15, -0.95 + initial.y * 0.15, 5.3)
            + (glider - initial) * Vec3::new(0.5, 0.5, 1.0)
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

#[derive(Resource, Clone)]
pub(crate) struct Level {
    pub(crate) number: usize,
    pub(crate) web: Web,
    pub(crate) flippers: Flippers,
}

#[derive(Clone)]
pub(crate) struct Flippers {
    pub(crate) count: u16,
    pub(crate) first_spawn_ticks: u16,
    pub(crate) spawn_interval_ticks: u16,
    pub(crate) climb_speed: i32,
    pub(crate) flip_speed: u16,
    pub(crate) pause_ticks: u8,
    pub(crate) max_active: usize,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            number: 1,
            web: Web {
                vertices: [
                    (7., 2.),
                    (9., 2.),
                    (11., 3.),
                    (13., 5.),
                    (14., 7.),
                    (14., 9.),
                    (13., 11.),
                    (11., 13.),
                    (9., 14.),
                    (7., 14.),
                    (5., 13.),
                    (3., 11.),
                    (2., 9.),
                    (2., 7.),
                    (3., 5.),
                    (5., 3.),
                ]
                .map(|(x, y)| [x, y])
                .to_vec(),
                orientations: (0..16).map(|lane| -lane * 16).collect(),
                start_lane: 7,
                depth: 160,
            },
            flippers: Flippers {
                count: 25,
                first_spawn_ticks: 61,
                spawn_interval_ticks: 121,
                climb_speed: 0xa000,
                flip_speed: 4,
                pause_ticks: 8,
                max_active: 11,
            },
        }
    }
}

#[derive(Resource)]
pub(crate) struct Levels(pub(crate) Vec<Level>);

impl Default for Levels {
    fn default() -> Self {
        Self(vec![Level::default()])
    }
}

#[derive(Clone, Copy)]
pub(crate) enum LoadReason {
    Restart,
    Retry,
    Advance,
}

#[derive(Component, Default)]
pub(crate) struct LevelActor;
