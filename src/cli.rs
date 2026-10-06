use clap::{ArgGroup, Parser};
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
mod tests {
    use super::*;
    use clap::{CommandFactory, error::ErrorKind};

    #[test]
    fn derive_definition_and_defaults_are_valid() {
        Cli::command().debug_assert();
        let cli = Cli::try_parse_from(["t2k"]).unwrap();
        assert_eq!(cli.level, 1);
        assert_eq!(cli.levels, PathBuf::from("levels"));
        assert!(cli.seconds.is_none());
    }

    #[test]
    fn existing_recording_syntax_accepts_duration_and_level_flags_in_either_order() {
        for flag in ["--record-motion", "--record-exit", "--record-shot-hit"] {
            for arguments in [
                vec!["t2k", "--level", "2", flag, "target/frames", "20"],
                vec!["t2k", flag, "target/frames", "20", "--level", "2"],
                vec!["t2k", flag, "target/frames", "--level", "2", "20"],
            ] {
                let cli = Cli::try_parse_from(arguments).unwrap();
                assert_eq!(cli.level, 2);
                assert_eq!(cli.seconds, Some(20));
            }
            assert!(
                Cli::try_parse_from(["t2k", flag, "target/frames"])
                    .unwrap()
                    .seconds
                    .is_none()
            );
        }
    }

    #[test]
    fn screenshots_combine_with_level_selection_and_custom_paths() {
        let cli = Cli::try_parse_from([
            "t2k",
            "--screenshot",
            "target/level 2.png",
            "--levels",
            "custom levels",
            "--level",
            "2",
        ])
        .unwrap();
        assert_eq!(cli.screenshot, Some(PathBuf::from("target/level 2.png")));
        assert_eq!(cli.levels, PathBuf::from("custom levels"));
        assert_eq!(cli.level, 2);
    }

    #[test]
    fn malformed_or_conflicting_capture_options_are_rejected() {
        for args in [
            vec!["t2k", "--level", "0"],
            vec!["t2k", "--record-motion"],
            vec!["t2k", "--record-motion", "frames", "abc"],
            vec!["t2k", "--record-motion", "frames", "0"],
            vec!["t2k", "--record-motion", "frames", "4294967295"],
            vec!["t2k", "20"],
            vec!["t2k", "--screenshot", "shot.png", "20"],
            vec!["t2k", "--record-motion", "frames", "--record-exit", "other"],
            vec![
                "t2k",
                "--screenshot",
                "shot.png",
                "--record-shot-hit",
                "frames",
            ],
            vec!["t2k", "--unknown"],
        ] {
            assert!(Cli::try_parse_from(&args).is_err(), "accepted {args:?}");
        }
        assert_eq!(
            Cli::try_parse_from(["t2k", "--help"]).unwrap_err().kind(),
            ErrorKind::DisplayHelp
        );
        assert_eq!(
            Cli::try_parse_from(["t2k", "--version"])
                .unwrap_err()
                .kind(),
            ErrorKind::DisplayVersion
        );
    }
}
