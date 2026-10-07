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

#[test]
fn level_23_keeps_the_tiny_stars_retraced_tips_and_lane_orientations() {
    let level = level(23);
    assert!(level.web.closed);
    assert_eq!(level.web.lanes(), 16);
    assert_eq!(level.web.start_lane, 9);
    assert_eq!(level.web.vertices[0], [8., 3.]);
    assert_eq!(level.web.vertices[3], level.web.vertices[5]);
    assert_eq!(level.web.vertices[7], level.web.vertices[9]);
    assert_eq!(level.web.orientations[3], 0);
    assert_eq!(level.web.orientations[4], -128);
    assert_eq!(level.flippers.climb_speed, 0xae00);
    assert_eq!(level.spikers.unwrap().climb_speed, 0xf000);
    assert_eq!(level.fuseballs.unwrap().climb_speed, 0xa800);
    assert_eq!(level.pulsars.unwrap().generator.spawn_interval_ticks, 463);
    assert_eq!(level.tankers.unwrap().super_chance, 88);
}

#[test]
fn level_24_has_the_closed_d_and_shorter_generator_delays() {
    let level = level(24);
    assert!(level.web.closed);
    assert_eq!(level.web.lanes(), 15);
    assert_eq!(level.web.start_lane, 12);
    assert_eq!(level.web.vertices[0], [2., 12.]);
    assert_eq!(level.web.vertices[9], [14., 12.]);
    assert_eq!(&level.web.orientations[9..], &[-128; 6]);
    assert_eq!(level.tankers.unwrap().spawn_interval_ticks, 241);
    assert_eq!(level.spikers.unwrap().spawn_interval_ticks, 651);
    let pulsars = level.pulsars.unwrap();
    assert_eq!(pulsars.generator.first_spawn_ticks, 206);
    assert_eq!(pulsars.generator.spawn_interval_ticks, 412);
    assert_eq!(level.flippers.count, 36);
}

#[test]
fn level_25_introduces_fuseball_tankers_on_the_curved_l() {
    let level = level(25);
    assert!(!level.web.closed);
    assert_eq!(level.web.lanes(), 13);
    assert_eq!(level.web.start_lane, 4);
    assert_eq!(level.web.vertices[0], [15., 13.]);
    assert_eq!(level.web.vertices[13], [-2., 0.]);
    assert_eq!(level.flippers.count, 41);
    assert_eq!(level.flippers.climb_speed, 0xb000);
    let tankers = level.fuse_tankers.unwrap();
    assert_eq!(tankers.first_spawn_ticks, 1001);
    assert_eq!(tankers.spawn_interval_ticks, 2001);
    assert_eq!(tankers.climb_speed, 0x9000);
    assert_eq!(level.pulsars.unwrap().phase_ticks, 7);
    assert_eq!(level.powerups.kill_interval, 8);
    let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/25.toml"));
    let invalid = source.replace(
        "[fuseballs]\nfirst_spawn_ticks = 351\nspawn_interval_ticks = 701\nclimb_speed = 0.6875\n",
        "",
    );
    assert!(
        Level::parse(&invalid, "invalid")
            .err()
            .unwrap()
            .contains("released children")
    );
}

#[test]
fn level_26_preserves_the_open_mouse_pointers_seventeen_lanes() {
    let level = level(26);
    assert!(!level.web.closed);
    assert_eq!(level.web.lanes(), 17);
    assert_eq!(level.web.start_lane, 13);
    assert_eq!(level.web.vertices[0], [-1., 13.]);
    assert_eq!(level.web.vertices[17], [2., 16.]);
    assert_eq!(level.web.orientations[2], -128);
    assert_eq!(level.web.orientations[3], 16);
    assert_eq!(level.pulsars.unwrap().generator.spawn_interval_ticks, 400);
    assert_eq!(level.spikers.unwrap().spawn_interval_ticks, 851);
    assert_eq!(level.fuse_tankers.unwrap().spawn_interval_ticks, 2001);
}

#[test]
fn level_27_keeps_both_open_curves_and_the_next_speed_group() {
    let level = level(27);
    assert!(!level.web.closed);
    assert_eq!(level.web.lanes(), 12);
    assert_eq!(level.web.start_lane, 7);
    assert_eq!(level.web.vertices[0], [9., 16.]);
    assert_eq!(level.web.vertices[12], [-3., 4.]);
    assert_eq!(level.web.orientations[5], 48);
    assert_eq!(level.web.orientations[6], -112);
    assert_eq!(level.flippers.climb_speed, 0xb200);
    assert_eq!(level.spikers.unwrap().climb_speed, 0x11000);
    assert_eq!(level.fuseballs.unwrap().climb_speed, 0xb800);
    assert_eq!(level.pulsars.unwrap().generator.spawn_interval_ticks, 389);
    assert_eq!(level.tankers.unwrap().spawn_interval_ticks, 371);
}
