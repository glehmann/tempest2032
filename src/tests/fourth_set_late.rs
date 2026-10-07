//! Original dispatch types and wave settings for the latter half of the fourth set.

use super::Levels;
use std::path::Path;

#[rustfmt::skip]
const WAVES: &[(usize, u16, u16, u16, u16, u16, u16, u16, u16)] = &[
    // Level, budget, Super Flippers, Mirrors, Spikers, Pulsars, Pulsar Tankers, Fuseballs, Beasties.
    (57, 61, 26, 401, 0, 0, 0, 0, 0),
    (58, 41, 0, 401, 51, 152, 0, 0, 0),
    (59, 41, 0, 401, 51, 152, 0, 524, 0),
    (60, 51, 0, 401, 51, 152, 0, 524, 0),
];

#[test]
fn fourth_set_late_uses_the_original_dispatch_types_and_wave_intervals() {
    let levels = Levels::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("levels")).unwrap();
    for &(number, count, supers, mirrors, spikers, pulsars, pulsar_tankers, fuses, beasts) in WAVES
    {
        let level = &levels.0[number - 1];
        let earlier = &levels.0[number - 33];
        assert_eq!(level.web.vertices, earlier.web.vertices);
        assert_eq!(level.web.orientations, earlier.web.orientations);
        assert_eq!(level.web.start_lane, earlier.web.start_lane);
        assert_eq!(level.web.closed, earlier.web.closed);
        assert!(!level.flippers.enabled);
        assert_eq!(
            level
                .extra_spikers
                .as_ref()
                .map_or(0, |g| g.spawn_interval_ticks),
            if (60..=63).contains(&number) { 1501 } else { 0 }
        );
        assert_eq!(level.flippers.count, count);
        assert_eq!(
            level
                .super_flippers
                .as_ref()
                .map_or(0, |g| g.spawn_interval_ticks),
            supers
        );
        assert_eq!(
            level.mirrors.as_ref().map_or(0, |g| g.spawn_interval_ticks),
            mirrors
        );
        assert_eq!(
            level.spikers.as_ref().map_or(0, |g| g.spawn_interval_ticks),
            spikers
        );
        assert_eq!(
            level
                .pulsars
                .as_ref()
                .map_or(0, |p| p.generator.spawn_interval_ticks),
            pulsars
        );
        assert_eq!(
            level
                .pulsar_tankers
                .as_ref()
                .map_or(0, |p| p.generator.spawn_interval_ticks),
            pulsar_tankers
        );
        assert_eq!(
            level
                .fuseballs
                .as_ref()
                .map_or(0, |g| g.spawn_interval_ticks),
            fuses
        );
        assert_eq!(
            level.beasts.as_ref().map_or(0, |g| g.spawn_interval_ticks),
            beasts
        );
        if let Some(mirror) = &level.mirrors {
            assert_eq!(
                mirror.climb_speed,
                [0xc000, 0xc800, 0xd000, 0xd800][(number - 57) / 2]
            );
        }
        assert_eq!(level.appearance.flipper, [68, 236, 255]);
        assert_eq!(level.powerups.kill_interval, 9);
    }
}
