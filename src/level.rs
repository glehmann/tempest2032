//! Editable TOML level definitions, validation, enemy-generator settings,
//! the level catalog, and level-owned actor cleanup markers.

use crate::web::LANE_UNIT;
use crate::web::Web;
use bevy::prelude::*;
use serde::Deserialize;
use std::path::Path;
use std::sync::LazyLock;

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
    pub(crate) pulsars: Option<Pulsars>,
}

#[derive(Clone)]
pub(crate) struct Pulsars {
    pub(crate) generator: Generator,
    pub(crate) phase_ticks: u8,
    pub(crate) propagation_ticks: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PulsarDefinition {
    first_spawn_ticks: u16,
    spawn_interval_ticks: u16,
    climb_speed: f32,
    phase_ticks: u8,
    propagation_ticks: u8,
}

impl PulsarDefinition {
    fn compile(self, name: &str) -> Result<Pulsars, String> {
        if self.phase_ticks == 0 || self.propagation_ticks == 0 {
            return Err(format!(
                "{name}: Pulsar phase and propagation times must be positive"
            ));
        }
        Ok(Pulsars {
            generator: GeneratorDefinition {
                first_spawn_ticks: self.first_spawn_ticks,
                spawn_interval_ticks: self.spawn_interval_ticks,
                climb_speed: self.climb_speed,
                super_chance: 0,
            }
            .compile(name)?,
            phase_ticks: self.phase_ticks,
            propagation_ticks: self.propagation_ticks,
        })
    }
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
    pub(crate) first_spawn_ticks: u16,
    pub(crate) spawn_interval_ticks: u16,
    pub(crate) climb_speed: f32,
    #[serde(default)]
    pub(crate) super_chance: u8,
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
            Level::parse(
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/01.toml")),
                "levels/01.toml",
            )
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
    pub(crate) number: usize,
    #[serde(default)]
    pub(crate) enemy_count: Option<u16>,
    #[serde(default)]
    pub(crate) enemy_shots: Option<usize>,
    pub(crate) web: Web,
    pub(crate) flippers: FlipperDefinition,
    #[serde(default)]
    pub(crate) appearance: Appearance,
    #[serde(default)]
    pub(crate) powerups: PowerUps,
    #[serde(default)]
    pub(crate) tankers: Option<GeneratorDefinition>,
    #[serde(default)]
    pub(crate) spikers: Option<GeneratorDefinition>,
    #[serde(default)]
    pub(crate) beasts: Option<GeneratorDefinition>,
    #[serde(default)]
    pub(crate) fuseballs: Option<GeneratorDefinition>,
    #[serde(default)]
    pulsars: Option<PulsarDefinition>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FlipperDefinition {
    #[serde(default)]
    pub(crate) count: Option<u16>,
    pub(crate) first_spawn_ticks: u16,
    pub(crate) spawn_interval_ticks: u16,
    pub(crate) climb_speed: f32,
    pub(crate) flip_speed: u16,
    pub(crate) pause_ticks: u8,
    pub(crate) max_active: usize,
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
            pulsars: def.pulsars.map(|g| g.compile(name)).transpose()?,
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
#[path = "tests/level.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/later_levels.rs"]
mod later_levels;
