//! Original third-set waves, geometry reuse, and the Pulsar Tanker settings.

use super::Levels;
use std::path::Path;

#[test]
fn third_set_matches_original_waves_and_reuses_the_first_set_webs() {
    let levels = Levels::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("levels")).unwrap();
    for (number, count, flippers, tankers) in [(33, 25, 101, 501)] {
        let level = &levels.0[number - 1];
        let earlier = &levels.0[number - 33];
        assert_eq!(level.web.vertices, earlier.web.vertices);
        assert_eq!(level.web.closed, earlier.web.closed);
        assert_eq!(level.flippers.count, count);
        assert_eq!(level.flippers.spawn_interval_ticks, flippers);
        let tanker = level.pulsar_tankers.as_ref().unwrap();
        assert_eq!(tanker.generator.spawn_interval_ticks, tankers);
        assert_eq!(tanker.generator.first_spawn_ticks, (tankers - 1) / 2 + 1);
        assert_eq!(
            level.appearance.surface,
            [
                crate::cry::RED[15][15],
                crate::cry::GREEN[15][15],
                crate::cry::BLUE[15][15]
            ]
            .map(|channel| f32::from(channel) / 255.)
        );
        assert_eq!(
            level.appearance.flipper,
            [
                crate::cry::RED[12][11],
                crate::cry::GREEN[12][11],
                crate::cry::BLUE[12][11]
            ]
        );
        assert_eq!(level.powerups.kill_interval, 9);
        assert_eq!(level.enemy_shots, 4);
    }
}

#[test]
fn level_33_has_no_direct_pulsar_generator_and_uses_the_original_start_lane() {
    let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/33.toml"));
    let level = super::Level::parse(source, "level33").unwrap();
    assert_eq!(level.web.start_lane, 8);
    assert!(level.pulsars.is_none());
    assert!(level.tankers.is_none());
    assert!(level.fuse_tankers.is_none());
    assert_eq!(level.flippers.climb_speed, 0xb000);
    assert_eq!(level.pulse_settings().unwrap().phase_ticks, 7);
    assert_eq!(level.beasts.unwrap().spawn_interval_ticks, 701);
    let invalid = format!(
        "{source}\n[pulsars]\nfirst_spawn_ticks = 1\nspawn_interval_ticks = 1\nclimb_speed = 0.5\nphase_ticks = 6\npropagation_ticks = 5\n"
    );
    assert!(
        super::Level::parse(&invalid, "invalid")
            .err()
            .unwrap()
            .contains("must share")
    );
}
