use super::*;
use serde::Deserialize;
use std::{path::Path, sync::LazyLock};

#[derive(Clone, Deserialize)]
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
    pub(crate) appearance: Appearance,
    pub(crate) powerups: PowerUps,
    pub(crate) tankers: Option<Generator>,
    pub(crate) spikers: Option<Generator>,
    pub(crate) enemy_shots: usize,
    pub(crate) beasts: Option<Generator>,
    pub(crate) fuseballs: Option<Generator>,
}

fn closed_web() -> bool {
    true
}

#[derive(Clone)]
pub(crate) struct Generator {
    pub(crate) first_spawn_ticks: u16,
    pub(crate) spawn_interval_ticks: u16,
    pub(crate) climb_speed: i32,
    pub(crate) super_chance: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneratorDefinition {
    first_spawn_ticks: u16,
    spawn_interval_ticks: u16,
    climb_speed: f32,
    #[serde(default)]
    super_chance: u8,
}

impl GeneratorDefinition {
    fn compile(self, name: &str) -> Result<Generator, String> {
        if self.first_spawn_ticks == 0
            || self.spawn_interval_ticks == 0
            || !self.climb_speed.is_finite()
            || !(1.0 / 65536.0..=16.0).contains(&self.climb_speed)
        {
            return Err(format!(
                "{name}: enemy spawn times and climbing speed must be positive and finite"
            ));
        }
        Ok(Generator {
            first_spawn_ticks: self.first_spawn_ticks,
            spawn_interval_ticks: self.spawn_interval_ticks,
            climb_speed: (self.climb_speed * LANE_UNIT as f32).round() as i32,
            super_chance: self.super_chance,
        })
    }
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
        static FIRST: LazyLock<Level> = LazyLock::new(|| {
            Level::parse(include_str!("../levels/01.toml"), "levels/01.toml")
                .expect("bundled level one must be valid")
        });
        FIRST.clone()
    }
}

#[derive(Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Appearance {
    pub(crate) surface: [f32; 3],
    pub(crate) lines: [f32; 3],
    pub(crate) flipper: [u8; 3],
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            surface: [0., 0., 1.],
            lines: [0.015, 0.01, 0.78],
            flipper: [240, 0, 0],
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct PowerUps {
    pub(crate) kill_interval: u8,
}
impl Default for PowerUps {
    fn default() -> Self {
        Self { kill_interval: 5 }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    number: usize,
    #[serde(default)]
    enemy_count: Option<u16>,
    #[serde(default)]
    enemy_shots: Option<usize>,
    web: Web,
    flippers: FlipperDefinition,
    #[serde(default)]
    appearance: Appearance,
    #[serde(default)]
    powerups: PowerUps,
    #[serde(default)]
    tankers: Option<GeneratorDefinition>,
    #[serde(default)]
    spikers: Option<GeneratorDefinition>,
    #[serde(default)]
    beasts: Option<GeneratorDefinition>,
    #[serde(default)]
    fuseballs: Option<GeneratorDefinition>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FlipperDefinition {
    #[serde(default)]
    count: Option<u16>,
    first_spawn_ticks: u16,
    spawn_interval_ticks: u16,
    climb_speed: f32,
    flip_speed: u16,
    pause_ticks: u8,
    max_active: usize,
}

impl Level {
    pub(crate) fn parse(source: &str, name: &str) -> Result<Self, String> {
        let def: Definition = toml::from_str(source).map_err(|err| format!("{name}: {err}"))?;
        let fail = |message: &str| Err(format!("{name}: {message}"));
        let web = &def.web;
        if def.number == 0 {
            return fail("number must be positive");
        }
        let enemy_shots = def.enemy_shots.unwrap_or(((def.number - 1) / 8).min(3) + 1);
        if !(1..=16).contains(&enemy_shots) {
            return fail("enemy_shots must be between 1 and 16");
        }
        if !(if web.closed { 3 } else { 1 }..=128).contains(&web.lanes()) {
            return fail("web must have 1 to 128 lanes, at least 3 when closed");
        }
        if web.start_lane >= web.lanes() {
            return fail("start_lane is outside the web");
        }
        if !(1..=1000).contains(&web.depth) {
            return fail("depth must be between 1 and 1000");
        }
        if web.orientations.len() != web.lanes()
            || web
                .orientations
                .iter()
                .any(|angle| !(-256..=256).contains(angle))
        {
            return fail("orientations must contain one angle per lane, between -256 and 256");
        }
        let mut area = 0.0;
        for index in 0..web.lanes() {
            let a = Vec2::from_array(web.vertices[index]);
            let b = Vec2::from_array(web.vertices[(index + 1) % web.vertices.len()]);
            if !a.is_finite() || !b.is_finite() || a.distance_squared(b) < 1e-6 {
                return fail("web vertices must be finite with nonzero lane lengths");
            }
            area += a.perp_dot(b);
        }
        if !area.is_finite() || (web.closed && area <= 0.0) {
            return fail("web vertices must enclose an area in clockwise screen order");
        }
        let f = def.flippers;
        if def.tankers.is_some() && web.lanes() < 3 {
            return fail("Tankers require at least three lanes");
        }
        let count = def
            .enemy_count
            .or(f.count)
            .ok_or_else(|| format!("{name}: enemy_count or flippers.count is required"))?;
        if count == 0
            || f.first_spawn_ticks == 0
            || f.spawn_interval_ticks == 0
            || f.max_active == 0
        {
            return fail("Flipper count, spawn times, and max_active must be positive");
        }
        if !f.climb_speed.is_finite() || !(1.0 / 65536.0..=16.0).contains(&f.climb_speed) {
            return fail("climb_speed must be finite and between 1/65536 and 16 units per tick");
        }
        if !(1..=64).contains(&f.flip_speed) {
            return fail("flip_speed must be between 1 and 64 angle units per tick");
        }
        if def.powerups.kill_interval == 0 {
            return fail("kill_interval must be positive");
        }
        if def
            .appearance
            .surface
            .iter()
            .chain(&def.appearance.lines)
            .any(|channel| !channel.is_finite() || !(0.0..=1.0).contains(channel))
        {
            return fail("surface and line colors must have finite channels between 0 and 1");
        }
        Ok(Self {
            number: def.number,
            web: def.web,
            flippers: Flippers {
                count,
                first_spawn_ticks: f.first_spawn_ticks,
                spawn_interval_ticks: f.spawn_interval_ticks,
                climb_speed: (f.climb_speed * LANE_UNIT as f32).round() as i32,
                flip_speed: f.flip_speed,
                pause_ticks: f.pause_ticks,
                max_active: f.max_active,
            },
            appearance: def.appearance,
            powerups: def.powerups,
            tankers: def
                .tankers
                .map(|generator| generator.compile(name))
                .transpose()?,
            spikers: def
                .spikers
                .map(|generator| generator.compile(name))
                .transpose()?,
            enemy_shots,
            beasts: def.beasts.map(|g| g.compile(name)).transpose()?,
            fuseballs: def.fuseballs.map(|g| g.compile(name)).transpose()?,
        })
    }
}

#[derive(Resource)]
pub(crate) struct Levels(pub(crate) Vec<Level>);

impl Default for Levels {
    fn default() -> Self {
        Self(vec![Level::default()])
    }
}

impl Levels {
    pub(crate) fn load(directory: &Path) -> Result<Self, String> {
        let entries = std::fs::read_dir(directory)
            .map_err(|err| format!("{}: {err}", directory.display()))?;
        let mut levels = Vec::new();
        for entry in entries {
            let path = entry.map_err(|err| err.to_string())?.path();
            if path.extension().is_none_or(|ext| ext != "toml") {
                continue;
            }
            let source = std::fs::read_to_string(&path)
                .map_err(|err| format!("{}: {err}", path.display()))?;
            levels.push((
                path.clone(),
                Level::parse(&source, &path.display().to_string())?,
            ));
        }
        if levels.is_empty() {
            return Err(format!("{}: no TOML levels found", directory.display()));
        }
        levels.sort_by_key(|(_, level)| level.number);
        for (index, (path, level)) in levels.iter().enumerate() {
            if level.number != index + 1 {
                return Err(format!(
                    "{}: expected level number {}, found {}",
                    path.display(),
                    index + 1,
                    level.number
                ));
            }
        }
        Ok(Self(levels.into_iter().map(|(_, level)| level).collect()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_level_five_uses_the_triangle_and_faster_climbers() {
        let level = Level::parse(include_str!("../levels/05.toml"), "level5").unwrap();
        assert!(level.web.closed);
        assert_eq!(level.web.lanes(), 18);
        assert_eq!(level.web.start_lane, 9);
        assert_eq!(level.web.flip_angle(5, 6, 1), 48);
        assert_eq!(level.web.flip_angle(17, 0, 1), 32);
        assert_eq!(level.flippers.climb_speed, 0xa400);
        assert_eq!(level.tankers.as_ref().unwrap().climb_speed, 0x8800);
        assert_eq!(level.spikers.as_ref().unwrap().climb_speed, 0xe000);
        assert_eq!(level.powerups.kill_interval, 6);
    }

    #[test]
    fn original_level_six_has_an_open_u_and_the_thirty_one_spawn_budget() {
        let level = Level::parse(include_str!("../levels/06.toml"), "level6").unwrap();
        assert!(!level.web.closed);
        assert_eq!(level.web.lanes(), 15);
        assert_eq!(level.flippers.count, 31);
        assert_eq!(
            (
                level.flippers.first_spawn_ticks,
                level.flippers.spawn_interval_ticks
            ),
            (48, 96)
        );
        assert_eq!(level.tankers.as_ref().unwrap().spawn_interval_ticks, 261);
        assert_eq!(level.web.neighbor(14, 1), None);
        assert_eq!(level.web.neighbor(0, -1), None);
        assert_eq!(level.web.flip_angle(6, 7, 1), 128);
        assert_eq!(level.web.flip_angle(2, 3, 1), 112);
    }

    #[test]
    fn original_level_seven_selects_the_t2k_lemon_and_its_speed_group() {
        let level = Level::parse(include_str!("../levels/07.toml"), "level7").unwrap();
        assert!(level.web.closed);
        assert_eq!(level.web.lanes(), 14);
        assert_eq!(level.web.start_lane, 5);
        assert_eq!(level.web.vertices[0], [12., 3.]);
        assert_eq!(level.web.vertices[7], [3., 12.]);
        assert_eq!(level.flippers.climb_speed, 0xa600);
        assert_eq!(level.spikers.as_ref().unwrap().climb_speed, 0xf000);
        assert_eq!(
            (
                level.tankers.as_ref().unwrap().first_spawn_ticks,
                level.tankers.as_ref().unwrap().spawn_interval_ticks
            ),
            (126, 251)
        );
        assert_eq!(level.web.neighbor(13, 1), Some(0));
        assert_eq!(level.web.flip_angle(13, 0, 1), 64);
    }

    #[test]
    fn original_level_eight_reuses_the_lemon_with_denser_tankers_and_spikers() {
        let seven = Level::parse(include_str!("../levels/07.toml"), "level7").unwrap();
        let eight = Level::parse(include_str!("../levels/08.toml"), "level8").unwrap();
        assert_eq!(seven.web.vertices, eight.web.vertices);
        assert_eq!(seven.web.orientations, eight.web.orientations);
        assert_eq!(eight.flippers.count, 31);
        assert_eq!(
            (
                eight.tankers.as_ref().unwrap().first_spawn_ticks,
                eight.tankers.as_ref().unwrap().spawn_interval_ticks
            ),
            (121, 241)
        );
        assert_eq!(
            (
                eight.spikers.as_ref().unwrap().first_spawn_ticks,
                eight.spikers.as_ref().unwrap().spawn_interval_ticks
            ),
            (326, 651)
        );
        assert_eq!(eight.tankers.as_ref().unwrap().super_chance, 28);
        assert!(
            Levels::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("levels"))
                .unwrap()
                .0
                .len()
                >= 8
        );
    }

    #[test]
    fn original_level_nine_has_the_sine_wave_and_two_enemy_shots() {
        let level = Level::parse(include_str!("../levels/09.toml"), "level9").unwrap();
        assert!(!level.web.closed);
        assert_eq!(level.web.lanes(), 14);
        assert_eq!(level.enemy_shots, 2);
        assert_eq!(level.flippers.climb_speed, 0xa800);
        assert_eq!(level.spikers.as_ref().unwrap().climb_speed, 0x10000);
        assert_eq!(level.tankers.as_ref().unwrap().spawn_interval_ticks, 231);
        assert_eq!(level.powerups.kill_interval, 7);
    }

    #[test]
    fn original_level_ten_has_the_clover_and_beast_generator() {
        let level = Level::parse(include_str!("../levels/10.toml"), "level10").unwrap();
        assert!(level.web.closed);
        assert_eq!(level.web.lanes(), 16);
        assert_eq!(level.web.vertices[0], [8., 6.]);
        assert_eq!(level.web.orientations[4], -16);
        assert_eq!(
            (
                level.beasts.as_ref().unwrap().first_spawn_ticks,
                level.beasts.as_ref().unwrap().spawn_interval_ticks
            ),
            (1001, 2001)
        );
    }

    #[test]
    fn original_level_twelve_has_the_cross_and_faster_fuseball_generator() {
        let level = Level::parse(include_str!("../levels/12.toml"), "level12").unwrap();
        assert!(level.web.closed);
        assert_eq!(level.web.lanes(), 16);
        assert_eq!(level.web.vertices[0], [7., 3.]);
        assert_eq!(level.flippers.climb_speed, 0xaa00);
        assert_eq!(level.fuseballs.as_ref().unwrap().spawn_interval_ticks, 701);
        assert_eq!(level.web.flip_angle(1, 2, 1), 160);
    }

    #[test]
    fn open_plane_has_eleven_lanes_and_never_wraps_at_its_ends() {
        let level = Level::parse(include_str!("../levels/03.toml"), "level3").unwrap();
        assert!(!level.web.closed);
        assert_eq!(level.web.lanes(), 11);
        assert_eq!(level.web.neighbor(0, -1), None);
        assert_eq!(level.web.neighbor(10, 1), None);
        assert_eq!(level.web.normalize(-LANE_UNIT), 0);
        assert_eq!(level.web.normalize(20 * LANE_UNIT), 10 * LANE_UNIT);
        assert_eq!(level.web.lane_delta(0, 10 * LANE_UNIT), 10 * LANE_UNIT);
        assert_eq!(level.web.vertex(11., FRONT_RADIUS, 0.).x, -22.0 / 3.0);
    }

    #[test]
    fn original_levels_have_the_expected_geometry_and_wave_settings() {
        let levels = Levels::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("levels")).unwrap();
        assert!(levels.0.len() >= 2);
        let first = &levels.0[0];
        let second = &levels.0[1];
        assert_eq!(
            (
                first.flippers.count,
                first.flippers.first_spawn_ticks,
                first.flippers.spawn_interval_ticks
            ),
            (25, 61, 121)
        );
        assert_eq!(
            (
                second.flippers.count,
                second.flippers.first_spawn_ticks,
                second.flippers.spawn_interval_ticks
            ),
            (25, 51, 101)
        );
        assert_eq!(second.flippers.climb_speed, 0xa000);
        assert_eq!(second.web.lanes(), 16);
        assert_eq!(
            second.web.glider_transform(second.web.start()).translation,
            Vec3::new(-2.0 / 3.0, -8.0 / 3.0, 0.12)
        );
        for lane in 0..16 {
            let next = (lane + 1) % 16;
            assert_eq!(
                second.web.flip_angle(lane, next, 1),
                if lane % 4 == 3 { 64 } else { 128 }
            );
            assert_eq!(
                second.web.flip_angle(next, lane, -1),
                if lane % 4 == 3 { 64 } else { 128 }
            );
            assert_eq!(first.web.flip_angle(lane, next, 1), 112);
        }
    }

    #[test]
    fn invalid_tweaks_report_the_file_and_reject_unknown_fields() {
        let source = include_str!("../levels/01.toml");
        for (old, new) in [
            ("start_lane = 7", "start_lane = 99"),
            ("depth = 160", "depth = 0"),
            ("first_spawn_ticks = 61", "first_spawn_ticks = 0"),
            ("climb_speed = 0.625", "climb_speed = nan"),
            ("flip_speed = 4", "flip_speed = 0"),
            ("kill_interval = 5", "kill_interval = 0"),
            ("spawn_interval_ticks = 121", "spawn_interva_ticks = 121"),
            ("[9, 2]", "[7, 2]"),
        ] {
            let error = Level::parse(&source.replace(old, new), "bad-level.toml")
                .err()
                .expect("invalid definition must fail");
            assert!(error.starts_with("bad-level.toml:"), "{error}");
        }
    }
}
