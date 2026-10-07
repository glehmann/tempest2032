//! Original second-set geometry and generator settings, starting at level 21.

use super::Levels;
use crate::level::Level;
use std::path::Path;

fn level(number: usize) -> Level {
    Levels::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("levels"))
        .unwrap()
        .0
        .remove(number - 1)
}

#[test]
fn level_21_reuses_the_compact_web_with_faster_pulsar_phases() {
    let level = level(21);
    assert!(level.web.closed);
    assert_eq!(level.web.lanes(), 8);
    assert_eq!(level.web.start_lane, 4);
    assert_eq!(level.web.flip_angle(7, 0, 1), 96);
    assert_eq!(level.flippers.count, 33);
    assert_eq!(level.flippers.climb_speed, 0xac00);
    assert_eq!(level.tankers.unwrap().super_chance, 80);
    let pulsars = level.pulsars.unwrap();
    assert_eq!(pulsars.generator.first_spawn_ticks, 217);
    assert_eq!(pulsars.generator.spawn_interval_ticks, 433);
    assert_eq!(pulsars.phase_ticks, 8);
    assert_eq!(level.spikers.unwrap().climb_speed, 0xe000);
    assert_eq!(level.fuseballs.unwrap().climb_speed, 0xa000);
}

#[test]
fn level_22_has_open_bird_wings_and_a_thirty_six_spawn_budget() {
    let level = level(22);
    assert!(!level.web.closed);
    assert_eq!(level.web.lanes(), 12);
    assert_eq!(level.web.start_lane, 6);
    assert_eq!(level.web.vertices[0], [18., 8.]);
    assert_eq!(level.web.vertices[12], [-2., 8.]);
    assert_eq!(level.web.neighbor(0, -1), None);
    assert_eq!(level.web.neighbor(11, 1), None);
    assert_eq!(level.flippers.count, 36);
    assert_eq!(level.flippers.first_spawn_ticks, 48);
    assert_eq!(level.flippers.spawn_interval_ticks, 96);
    assert_eq!(level.tankers.unwrap().spawn_interval_ticks, 261);
    let pulsars = level.pulsars.unwrap();
    assert_eq!(pulsars.generator.first_spawn_ticks, 213);
    assert_eq!(pulsars.generator.spawn_interval_ticks, 426);
    assert_eq!(pulsars.phase_ticks, 8);
}
