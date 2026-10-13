//! Clap options for gameplay, capture previews, and Steam installation.

use clap::ArgGroup;
use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(version, about = "Tempest 2032, a Tempest 2000-inspired desktop game")]
#[command(group(ArgGroup::new("recording").args(["record_motion", "record_exit", "record_shot_hit"])))]
#[command(group(ArgGroup::new("preview").args(["message_preview", "menu_preview"])))]
#[command(group(ArgGroup::new("game_options").multiple(true).args([
    "level", "levels", "fullscreen", "fps", "volume", "music_volume", "effects_pitch",
    "difficulty", "starfield", "warp_zone", "warp_course", "message_preview", "message_tick",
    "message_size", "menu_preview", "screenshot", "record_motion", "record_exit", "record_shot_hit", "seconds",
])))]
pub(crate) struct Cli {
    /// Copy this executable and add Tempest 2032 to Steam as a non-Steam game, then exit
    #[arg(long, conflicts_with = "game_options")]
    pub(crate) steam_install: bool,

    /// Steam data directory containing steamapps and userdata (otherwise detected)
    #[arg(long, value_name = "PATH", requires = "steam_install")]
    pub(crate) steam_dir: Option<PathBuf>,

    /// Steam account ID, as used in the userdata directory
    #[arg(long, value_name = "ACCOUNT_ID", requires = "steam_install", value_parser = clap::value_parser!(u32).range(1..))]
    pub(crate) steam_user: Option<u32>,

    /// Start on this level
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    pub(crate) level: Option<u32>,

    /// Directory containing TOML overrides for the embedded levels
    #[arg(long, default_value = "levels", value_name = "DIRECTORY")]
    pub(crate) levels: PathBuf,

    /// Start in borderless fullscreen instead of a window
    #[arg(long)]
    pub(crate) fullscreen: bool,

    /// Show the frame rate, processor load, and memory use in the top-right corner
    #[arg(long)]
    pub(crate) fps: bool,

    /// Master audio volume from 0 (silent) to 1 (full volume)
    #[arg(long, default_value_t = 0.6, value_parser = parse_volume)]
    pub(crate) volume: f32,

    /// Music volume relative to master audio, from 0 (off) to 1
    #[arg(long, default_value_t = 1.0, value_parser = parse_volume)]
    pub(crate) music_volume: f32,

    /// Gameplay-effect pitch multiplier, leaving voices unchanged (0 < value <= 4)
    #[arg(long, default_value_t = 2.0, value_parser = parse_effects_pitch)]
    pub(crate) effects_pitch: f32,

    /// Modulate difficulty without changing the level definitions
    #[arg(long, value_enum, value_name = "LEVEL")]
    pub(crate) difficulty: Option<crate::difficulty::Difficulty>,

    /// Background star animation (default: the original pattern for each level)
    #[arg(long, value_enum, value_name = "PATTERN")]
    pub(crate) starfield: Option<crate::stars::Pattern>,

    /// Start directly in a playable warp zone (gate-flight/1, tunnel/2, rings/3)
    #[arg(long, value_enum, value_name = "ZONE",conflicts_with_all=["record_exit","record_shot_hit"])]
    pub(crate) warp_zone: Option<crate::warp::ZoneKind>,

    /// Choose an original course for direct warp startup
    #[arg(long,requires="warp_zone",value_parser=clap::value_parser!(u8).range(1..=8))]
    pub(crate) warp_course: Option<u8>,

    /// Preview an original animated announcement or reward
    #[arg(long, value_enum, value_name = "MESSAGE", conflicts_with_all=["record_exit","record_shot_hit"])]
    pub(crate) message_preview: Option<crate::messages::Id>,

    /// Freeze the preview at this fixed animation tick (screenshots default to 60)
    #[arg(long, requires = "message_preview")]
    pub(crate) message_tick: Option<u16>,

    /// Exact pixel dimensions for a message or menu preview, e.g. 1280x800
    #[arg(long,requires="preview",conflicts_with="fullscreen",value_parser=parse_message_size,value_name="WIDTHxHEIGHT")]
    pub(crate) message_size: Option<bevy::prelude::UVec2>,

    /// Capture a front-end screen instead of gameplay
    #[arg(long, value_enum, requires = "screenshot", conflicts_with_all = ["message_preview", "level", "warp_zone"])]
    pub(crate) menu_preview: Option<crate::menu::Preview>,

    /// Save the settled level or warp scene to a screenshot and exit
    #[arg(long, value_name = "FILE", conflicts_with = "recording")]
    pub(crate) screenshot: Option<PathBuf>,

    /// Record scripted movement, default 6 seconds (30 for direct warp startup)
    #[arg(long, value_name = "DIRECTORY")]
    pub(crate) record_motion: Option<PathBuf>,

    /// Record a level-clear exit, default duration 14 seconds
    #[arg(long, value_name = "DIRECTORY")]
    pub(crate) record_exit: Option<PathBuf>,

    /// Record an enemy-shot death, default duration 15 seconds
    #[arg(long, value_name = "DIRECTORY")]
    pub(crate) record_shot_hit: Option<PathBuf>,

    /// Override the recording duration in whole seconds
    #[arg(value_name = "SECONDS", requires = "recording", value_parser = clap::value_parser!(u32).range(1..=u32::MAX as i64 / 60))]
    pub(crate) seconds: Option<u32>,
}

impl Cli {
    pub(crate) fn direct_start(&self) -> bool {
        self.level.is_some()
            || self.warp_zone.is_some()
            || self.message_preview.is_some()
            || self.screenshot.is_some()
            || self.record_motion.is_some()
            || self.record_exit.is_some()
            || self.record_shot_hit.is_some()
    }
}

fn parse_message_size(value: &str) -> Result<bevy::prelude::UVec2, String> {
    let (width, height) = value.split_once('x').ok_or("expected WIDTHxHEIGHT")?;
    let width: u32 = width.parse().map_err(|_| "invalid width")?;
    let height: u32 = height.parse().map_err(|_| "invalid height")?;
    if !(320..=8192).contains(&width) || !(240..=8192).contains(&height) {
        return Err("dimensions must be 320..8192 by 240..8192".into());
    }
    Ok(bevy::prelude::UVec2::new(width, height))
}

fn parse_volume(value: &str) -> Result<f32, String> {
    let volume: f32 = value.parse().map_err(|_| "expected a number from 0 to 1")?;
    if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
        return Err("volume must be from 0 to 1".into());
    }
    Ok(volume)
}

fn parse_effects_pitch(value: &str) -> Result<f32, String> {
    let pitch: f32 = value.parse().map_err(|_| "expected a pitch multiplier")?;
    if !pitch.is_finite() || !(0.0..=4.0).contains(&pitch) || pitch == 0.0 {
        return Err("effects pitch must be greater than 0 and at most 4".into());
    }
    Ok(pitch)
}

#[cfg(test)]
#[path = "tests/cli.rs"]
mod tests;
