//! Clap command-line options for level selection, custom catalogs, screenshots,
//! and scripted movement, exit, and shot-hit recordings.

use clap::ArgGroup;
use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(version, about = "Tempest 2000-inspired desktop prototype")]
#[command(group(ArgGroup::new("recording").args(["record_motion", "record_exit", "record_shot_hit"])))]
pub(crate) struct Cli {
    /// Start on this level
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..))]
    pub(crate) level: u32,

    /// Directory containing the TOML level definitions
    #[arg(long, default_value = "levels", value_name = "DIRECTORY")]
    pub(crate) levels: PathBuf,

    /// Start in borderless fullscreen instead of a window
    #[arg(long)]
    pub(crate) fullscreen: bool,

    /// Show the frame rate, processor load, and memory use in the top-right corner
    #[arg(long)]
    pub(crate) fps: bool,

    /// Save the settled level to a screenshot and exit
    #[arg(long, value_name = "FILE", conflicts_with = "recording")]
    pub(crate) screenshot: Option<PathBuf>,

    /// Record scripted movement, default duration 6 seconds
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

#[cfg(test)]
#[path = "tests/cli.rs"]
mod tests;
