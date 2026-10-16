//! Source wave intervals, speed groups, palettes, and final-wave aliases for levels 81–100.

use super::{Generator, Levels};
use std::path::Path;

#[rustfmt::skip]
#[allow(clippy::type_complexity)]
const WAVES: &[(usize,u16,u16,u16,u16,u16,u16,u16,u16)] = &[
    // Level, budget, Mirrors, Alien Droids, Pulsar Tankers, type 3, Flipper Tankers, Fuseball Tankers, Pulsars.
    (81, 61, 751, 601, 501, 401, 701, 624, 301),
    (82, 61, 751, 501, 501, 401, 501, 624, 301),
    (83, 61, 751, 501, 501, 401, 501, 624, 301),
    (84, 61, 751, 501, 501, 401, 501, 524, 301),
    (85, 71, 751, 501, 501, 401, 501, 524, 301),
    (86, 71, 751, 401, 501, 401, 501, 524, 301),
    (87, 71, 751, 401, 451, 401, 501, 524, 301),
    (88, 71, 751, 401, 451, 401, 501, 524, 301),
    (89, 71, 751, 401, 451, 401, 301, 524, 301),
    (90, 81, 751, 401, 451, 401, 301, 524, 301),
    (91, 81, 751, 401, 451, 351, 301, 524, 301),
    (92, 81, 751, 401, 451, 351, 251, 524, 301),
    (93, 81, 751, 401, 451, 351, 251, 524, 101),
    (94, 81, 751, 401, 451, 351, 251, 524, 101),
    (95, 81, 751, 201, 451, 351, 251, 524, 101),
    (96, 91, 351, 201, 451, 351, 251, 524, 101),
    (97, 91, 351, 201, 451, 351, 251, 424, 101),
    (98, 91, 351, 201, 451, 351, 251, 324, 101),
    (99, 101, 0, 0, 0, 0, 0, 0, 0),
    (100, 101, 0, 0, 0, 0, 0, 0, 0),
];

fn generator(actual: Option<&Generator>, interval: u16, speed: i32) {
    assert_eq!(actual.map_or(0, |g| g.spawn_interval_ticks), interval);
    if let Some(g) = actual {
        assert_eq!(g.first_spawn_ticks, (interval - 1) / 2 + 1);
        assert_eq!(g.climb_speed, speed);
    }
}

#[test]
fn final_sets_match_original_waves_speeds_and_cry_palettes() {
    let levels = Levels::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("levels")).unwrap();
    for &(
        number,
        count,
        mirrors,
        aliens,
        pulsar_tankers,
        spirals,
        tankers,
        fuse_tankers,
        pulsars,
    ) in WAVES
    {
        let level = &levels.0[number - 1];
        let earlier = &levels.0[if number <= 96 {
            number - 65
        } else {
            number - 97
        }];
        let group = (number - 81) / 2;
        let flip_speed = [
            0xc000, 0xc200, 0xc400, 0xc600, 0xc800, 0xca00, 0xcc00, 0xce00, 0xd000, 0xd200,
        ][group];
        let fuse_speed = [
            0xc000, 0xc800, 0xd000, 0xd800, 0xe000, 0xe800, 0xf000, 0xf800, 0xd000, 0xe800,
        ][group];
        let tank_speed = [
            0xa000, 0xa000, 0xb800, 0xb800, 0xc000, 0xc000, 0xc800, 0xc800, 0xa000, 0xa000,
        ][group];
        assert_eq!(level.web.vertices, earlier.web.vertices);
        assert_eq!(level.web.closed, earlier.web.closed);
        assert!(!level.flippers.enabled);
        assert_eq!(level.flippers.count, count);
        generator(level.super_flippers.as_ref(), 21, flip_speed);
        generator(level.spiral_flippers.as_ref(), spirals, flip_speed);
        generator(level.mirrors.as_ref(), mirrors, fuse_speed);
        generator(level.aliens.as_ref(), aliens, flip_speed * 2);
        generator(level.tankers.as_ref(), tankers, tank_speed);
        generator(level.fuse_tankers.as_ref(), fuse_tankers, tank_speed);
        generator(
            level.pulsars.as_ref().map(|p| &p.generator),
            pulsars,
            0x8000,
        );
        generator(
            level.pulsar_tankers.as_ref().map(|p| &p.generator),
            pulsar_tankers,
            tank_speed,
        );
        generator(
            level.fuseballs.as_ref(),
            if number >= 99 { 0 } else { 301 },
            fuse_speed,
        );
        generator(
            level.beasts.as_ref(),
            if number >= 99 { 101 } else { 801 },
            flip_speed,
        );
        assert!(level.spikers.is_none());
        assert!(level.extra_spikers.is_none());
        if let Some(g) = &level.tankers {
            assert_eq!(g.super_chance, 252);
            assert_eq!(g.spiral_chance, 2 * (number as u8 - 1));
        }
        if let Some(p) = level.pulse_settings() {
            assert_eq!(
                p.phase_ticks,
                if number >= 97 {
                    7
                } else {
                    7 - (number - 81) as u8 / 4
                }
            );
            assert_eq!(p.propagation_ticks, 5);
        }
        let (web_hue, flip_hue) = if number >= 97 {
            (0x88, 0x80)
        } else {
            (0xf8, 0xfb)
        };
        let rgb = |hue: u8| {
            [
                crate::cry::RED[(hue >> 4) as usize][(hue & 15) as usize],
                crate::cry::GREEN[(hue >> 4) as usize][(hue & 15) as usize],
                crate::cry::BLUE[(hue >> 4) as usize][(hue & 15) as usize],
            ]
        };
        assert_eq!(
            level.appearance.surface,
            rgb(web_hue).map(|c| f32::from(c) / 255.)
        );
        assert_eq!(level.appearance.flipper, rgb(flip_hue));
        assert_eq!(
            level.powerups.kill_interval,
            if number >= 97 { 16 } else { 11 }
        );
    }
}

#[test]
fn the_final_two_levels_share_the_aliased_wave_but_use_distinct_webs() {
    let levels = Levels::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("levels")).unwrap();
    assert_eq!(levels.0.len(), 100);
    let a = &levels.0[98];
    let b = &levels.0[99];
    assert_ne!(a.web.vertices, b.web.vertices);
    assert_eq!(a.flippers.count, b.flippers.count);
    assert_eq!(
        a.super_flippers.as_ref().unwrap().spawn_interval_ticks,
        b.super_flippers.as_ref().unwrap().spawn_interval_ticks
    );
    assert_eq!(
        a.beasts.as_ref().unwrap().spawn_interval_ticks,
        b.beasts.as_ref().unwrap().spawn_interval_ticks
    );
    assert!(a.pulsars.is_none() && b.pulsars.is_none());
    assert!(a.mirrors.is_none() && b.mirrors.is_none());
}
