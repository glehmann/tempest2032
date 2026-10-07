//! Checks CLI defaults, capture combinations, duration syntax, help, and invalid arguments.

use crate::cli::Cli;
use clap::Parser;
use clap::{CommandFactory, error::ErrorKind};
use std::path::PathBuf;

#[test]
fn derive_definition_and_defaults_are_valid() {
    Cli::command().debug_assert();
    let cli = Cli::try_parse_from(["t2k"]).unwrap();
    assert_eq!(cli.level, 1);
    assert_eq!(cli.levels, PathBuf::from("levels"));
    assert!(cli.seconds.is_none());
    assert!(!cli.fullscreen);
    assert!(!cli.fps);
}

#[test]
fn fps_display_combines_with_level_and_capture_options() {
    let cli = Cli::try_parse_from(["t2k", "--fps", "--level", "3"]).unwrap();
    assert!(cli.fps);
    assert_eq!(cli.level, 3);
    assert!(
        Cli::try_parse_from(["t2k", "--fps", "--screenshot", "shot.png", "--fullscreen"])
            .unwrap()
            .fps
    );
}

#[test]
fn fullscreen_combines_with_level_and_capture_options() {
    let cli = Cli::try_parse_from(["t2k", "--fullscreen", "--level", "3"]).unwrap();
    assert!(cli.fullscreen);
    assert_eq!(cli.level, 3);
    assert!(
        Cli::try_parse_from(["t2k", "--screenshot", "shot.png", "--fullscreen"])
            .unwrap()
            .fullscreen
    );
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
