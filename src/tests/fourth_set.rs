//! Original fourth-set waves, including the Beastie-only opening wave.

use super::Levels;
use std::path::Path;

#[rustfmt::skip]
#[allow(clippy::type_complexity)]
const WAVES: &[(usize, u16, bool, u16, u16, u16, u16, u16)] = &[
    // Level, budget, normal Flippers, Flipper delay, Beastie delay, Tankers, Fuseballs, Pulsar Tankers.
    (49, 17, false, 141, 141, 0, 0, 0),
    (50, 41, true, 51, 501, 0, 901, 0),
    (51, 46, true, 51, 501, 0, 0, 951),
    (52, 51, true, 51, 301, 0, 951, 0),
    (53, 51, true, 51, 201, 281, 901, 0),
    (54, 51, true, 51, 101, 261, 901, 0),
    (55, 51, true, 51, 101, 251, 901, 0),
    (56, 51, true, 51, 101, 241, 901, 0),
];

#[test]
fn fourth_set_matches_the_original_waves_and_second_set_geometry() {
    let levels = Levels::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("levels")).unwrap();
    for &(number, count, flippers, flip_delay, beast_delay, tankers, fuses, pulsar_tankers) in WAVES
    {
        let level = &levels.0[number - 1];
        let earlier = &levels.0[number - 33];
        assert_eq!(level.web.vertices, earlier.web.vertices);
        assert_eq!(level.web.orientations, earlier.web.orientations);
        assert_eq!(level.web.closed, earlier.web.closed);
        assert_eq!(level.web.start_lane, earlier.web.start_lane);
        assert_eq!(level.flippers.count, count);
        assert_eq!(level.flippers.enabled, flippers);
        assert_eq!(level.flippers.spawn_interval_ticks, flip_delay);
        assert_eq!(level.flippers.first_spawn_ticks, (flip_delay - 1) / 2 + 1);
        let beasts = level.beasts.as_ref().unwrap();
        assert_eq!(beasts.spawn_interval_ticks, beast_delay);
        assert_eq!(beasts.first_spawn_ticks, (beast_delay - 1) / 2 + 1);
        assert_eq!(
            level.tankers.as_ref().map_or(0, |g| g.spawn_interval_ticks),
            tankers
        );
        assert_eq!(
            level
                .fuseballs
                .as_ref()
                .map_or(0, |g| g.spawn_interval_ticks),
            fuses
        );
        assert_eq!(
            level
                .pulsar_tankers
                .as_ref()
                .map_or(0, |p| p.generator.spawn_interval_ticks),
            pulsar_tankers
        );
        assert!(level.pulsars.is_none());
        assert!(level.fuse_tankers.is_none());
        if let Some(tankers) = &level.tankers {
            assert_eq!(tankers.super_chance, 4 * (number as u8 - 1));
            assert_eq!(
                tankers.climb_speed,
                [0x8000, 0x8000, 0x9800, 0x9800][(number - 49) / 2]
            );
        }
        if let Some(fuses) = &level.fuseballs {
            assert_eq!(
                fuses.climb_speed,
                [0xa000, 0xa800, 0xb000, 0xb800][(number - 49) / 2]
            );
        }
        if let Some(spikers) = &level.spikers {
            assert_eq!(
                spikers.climb_speed,
                [0xd000, 0xd000, 0xe000, 0xf000][(number - 49) / 2]
            );
        }
        if let Some(pulsars) = &level.pulsar_tankers {
            assert_eq!(pulsars.phase_ticks, 7);
            assert_eq!(pulsars.propagation_ticks, 5);
            assert_eq!(pulsars.generator.climb_speed, 0x8000);
        }
        assert_eq!(
            beasts.climb_speed,
            [0xb000, 0xb200, 0xb400, 0xb600][(number - 49) / 2]
        );
        assert_eq!(
            level.appearance.surface,
            [
                crate::cry::RED[0][1],
                crate::cry::GREEN[0][1],
                crate::cry::BLUE[0][1]
            ]
            .map(|channel| f32::from(channel) / 255.)
        );
        assert_eq!(
            level.appearance.flipper,
            [
                crate::cry::RED[2][11],
                crate::cry::GREEN[2][11],
                crate::cry::BLUE[2][11]
            ]
        );
        assert_eq!(level.enemy_shots, 4);
        assert_eq!(level.powerups.kill_interval, 9);
    }
}

#[test]
fn disabling_flippers_requires_another_generator_and_defaults_to_enabled() {
    assert!(super::Level::default().flippers.enabled);
    let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/01.toml"));
    let invalid = source.replace("[flippers]", "[flippers]\nenabled = false");
    assert!(
        super::Level::parse(&invalid, "invalid")
            .err()
            .unwrap()
            .contains("another enemy generator")
    );
}
