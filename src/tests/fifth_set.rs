//! Fifth-set dispatch types, speed groups, geometry, and Jaguar CRY palettes.

use super::Levels;
use std::path::Path;

#[rustfmt::skip]
const WAVES: &[(usize,u16,u16,u16,u16,u16,u16,u16)] = &[
    // Level, Super Flippers, Mirrors, Alien Droids, Fuseballs, Pulsar Tankers, Beasties, Spikers.
    (65, 21, 0, 701, 0, 0, 0, 0),
    (66, 41, 751, 601, 0, 0, 0, 0),
];

#[test]
fn fifth_set_matches_the_original_dispatch_table_and_speed_groups() {
    let levels = Levels::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("levels")).unwrap();
    for &(number, supers, mirrors, aliens, fuses, pulsar_tankers, beasts, spikers) in WAVES {
        let level = &levels.0[number - 1];
        let earlier = &levels.0[number - 65];
        assert_eq!(level.web.vertices, earlier.web.vertices);
        assert_eq!(level.web.closed, earlier.web.closed);
        assert_eq!(level.flippers.count, 61);
        assert!(!level.flippers.enabled);
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
        assert_eq!(
            level.beasts.as_ref().map_or(0, |g| g.spawn_interval_ticks),
            beasts
        );
        assert_eq!(
            level.spikers.as_ref().map_or(0, |g| g.spawn_interval_ticks),
            spikers
        );
        assert!(level.pulsars.is_none());
        assert!(level.extra_spikers.is_none());
        let droids = level.aliens.as_ref().unwrap();
        assert_eq!(droids.spawn_interval_ticks, aliens);
        assert_eq!(droids.first_spawn_ticks, (aliens - 1) / 2 + 1);
        assert_eq!(
            droids.climb_speed,
            2 * [0xb000, 0xb200, 0xb400, 0xb600][(number - 65) / 2]
        );
        assert_eq!(
            level.appearance.surface,
            [
                crate::cry::RED[10][0],
                crate::cry::GREEN[10][0],
                crate::cry::BLUE[10][0]
            ]
            .map(|channel| f32::from(channel) / 255.)
        );
        assert_eq!(
            level.appearance.flipper,
            [
                crate::cry::RED[8][8],
                crate::cry::GREEN[8][8],
                crate::cry::BLUE[8][8]
            ]
        );
    }
}
