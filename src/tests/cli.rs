//! Checks CLI defaults, capture combinations, duration syntax, help, and invalid arguments.

use crate::cli::Cli;
use clap::Parser;

#[test]
fn direct_warp_selection_validates_zone_course_and_capture_options() {
    let args = crate::cli::Cli::try_parse_from([
        "t2k",
        "--warp-zone",
        "gate-flight",
        "--warp-course",
        "8",
        "--level",
        "65",
        "--fps",
        "--screenshot",
        "warp.png",
    ])
    .unwrap();
    assert_eq!(args.warp_zone, Some(crate::warp::ZoneKind::GateFlight));
    assert_eq!(args.warp_course, Some(8));
    assert!(crate::cli::Cli::try_parse_from(["t2k", "--warp-zone", "1"]).is_ok());
    for zone in ["tunnel", "green-track", "2"] {
        assert_eq!(
            Cli::try_parse_from(["t2k", "--warp-zone", zone])
                .unwrap()
                .warp_zone,
            Some(crate::warp::ZoneKind::Tunnel)
        );
    }
    for zone in ["rings", "psychedelic-rings", "3"] {
        assert_eq!(
            Cli::try_parse_from(["t2k", "--warp-zone", zone, "--warp-course", "8"])
                .unwrap()
                .warp_zone,
            Some(crate::warp::ZoneKind::Rings)
        );
    }
    for args in [
        vec!["t2k", "--warp-zone", "gate-flight", "--record-exit", "out"],
        vec![
            "t2k",
            "--warp-zone",
            "gate-flight",
            "--record-shot-hit",
            "out",
        ],
        vec!["t2k", "--warp-course", "1"],
        vec!["t2k", "--warp-zone", "unknown"],
        vec!["t2k", "--warp-zone", "gate", "--warp-course", "0"],
        vec!["t2k", "--warp-zone", "gate", "--warp-course", "9"],
    ] {
        assert!(crate::cli::Cli::try_parse_from(args).is_err());
    }
}
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
    assert_eq!(cli.volume, 0.6);
}

#[test]
fn sound_volume_accepts_silence_and_rejects_nonfinite_or_out_of_range_values() {
    for value in ["0", "0.25", "1"] {
        assert_eq!(
            Cli::try_parse_from(["t2k", "--volume", value])
                .unwrap()
                .volume,
            value.parse::<f32>().unwrap()
        );
    }
    for value in ["NaN", "inf", "1.01", "-0.1", "loud"] {
        assert!(Cli::try_parse_from(["t2k", "--volume", value]).is_err());
    }
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
#[test]
fn message_previews_validate_identifiers_ticks_sizes_and_capture_modes() {
    let cli = Cli::try_parse_from([
        "t2k",
        "--message-preview",
        "particle-laser",
        "--message-tick",
        "130",
        "--message-size",
        "3840x2160",
        "--screenshot",
        "stretch.png",
    ])
    .unwrap();
    assert_eq!(
        cli.message_preview,
        Some(crate::messages::Id::ParticleLaser)
    );
    assert_eq!(cli.message_tick, Some(130));
    assert_eq!(
        cli.message_size,
        Some(bevy::prelude::UVec2::new(3840, 2160))
    );
    for arguments in [
        vec!["t2k", "--message-tick", "10"],
        vec!["t2k", "--message-preview", "unknown"],
        vec![
            "t2k",
            "--message-preview",
            "excellent",
            "--message-size",
            "0x2160",
        ],
        vec![
            "t2k",
            "--message-preview",
            "excellent",
            "--message-size",
            "3840",
        ],
        vec![
            "t2k",
            "--message-preview",
            "excellent",
            "--record-exit",
            "frames",
        ],
    ] {
        assert!(Cli::try_parse_from(arguments).is_err());
    }
}
#[test]
fn original_starfield_variants_can_be_selected_with_levels_and_capture() {
    for pattern in ["level", "rings", "selection"] {
        assert!(
            Cli::try_parse_from([
                "t2k",
                "--starfield",
                pattern,
                "--level",
                "32",
                "--screenshot",
                "stars.png"
            ])
            .is_ok()
        );
    }
    assert!(Cli::try_parse_from(["t2k", "--starfield", "unknown"]).is_err());
    assert!(Cli::try_parse_from(["t2k", "--starfield", "random"]).is_err());
}

#[test]
fn difficulty_defaults_to_normal_and_accepts_the_three_settings() {
    use crate::difficulty::Difficulty;
    assert_eq!(Cli::parse_from(["t2k"]).difficulty, Difficulty::Normal);
    for (name, expected) in [
        ("low", Difficulty::Low),
        ("normal", Difficulty::Normal),
        ("high", Difficulty::High),
    ] {
        assert_eq!(
            Cli::try_parse_from(["t2k", "--difficulty", name])
                .unwrap()
                .difficulty,
            expected
        );
        assert!(Cli::try_parse_from(["t2k", "--difficulty", name, "--level", "3"]).is_ok());
    }
    assert!(Cli::try_parse_from(["t2k", "--difficulty", "beastly"]).is_err());
}
