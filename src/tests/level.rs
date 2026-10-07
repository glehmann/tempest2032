//! Checks original level geometry and wave settings, open-web topology, and TOML validation.

use crate::level::Level;
use crate::level::Levels;
use crate::web::FRONT_RADIUS;
use crate::web::LANE_UNIT;
use bevy::prelude::*;

#[test]
fn original_level_nineteen_has_the_pentagon_and_restores_flipper_tankers() {
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/19.toml")),
        "level19",
    )
    .unwrap();
    assert!(level.web.closed);
    assert_eq!(level.web.lanes(), 15);
    assert_eq!(level.web.start_lane, 7);
    assert_eq!(level.web.vertices[0], [8., 2.]);
    assert_eq!(level.web.vertices[7], [9., 14.]);
    assert_eq!(level.web.neighbor(14, 1), Some(0));
    assert_eq!(level.web.flip_angle(2, 3, 1), 80);
    assert_eq!(level.flippers.count, 31);
    assert_eq!(level.flippers.first_spawn_ticks, 61);
    assert_eq!(level.flippers.spawn_interval_ticks, 121);
    assert_eq!(level.flippers.climb_speed, 0xaa00);
    let tankers = level.tankers.unwrap();
    assert_eq!(tankers.first_spawn_ticks, 223);
    assert_eq!(tankers.spawn_interval_ticks, 445);
    assert_eq!(tankers.climb_speed, 0x8000);
    assert_eq!(tankers.super_chance, 72);
    let pulsars = level.pulsars.unwrap();
    assert_eq!(pulsars.generator.first_spawn_ticks, 301);
    assert_eq!(pulsars.generator.spawn_interval_ticks, 601);
    assert_eq!(pulsars.phase_ticks, 9);
    assert_eq!(level.spikers.unwrap().climb_speed, 0xd000);
    assert_eq!(level.beasts.unwrap().climb_speed, 0xaa00);
    assert_eq!(level.fuseballs.unwrap().climb_speed, 0x9800);
    assert_eq!(level.enemy_shots, 3);
    assert_eq!(level.appearance.surface, [1., 0., 0.]);
    assert_eq!(level.powerups.kill_interval, 7);
}

#[test]
fn original_level_eighteen_has_the_open_spiraloid_and_a_denser_pulsar_wave() {
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/18.toml")),
        "level18",
    )
    .unwrap();
    assert_eq!(level.web.lanes(), 15);
    assert!(!level.web.closed);
    assert_eq!(level.web.start_lane, 11);
    assert_eq!(level.web.vertices[0], [7., 6.]);
    assert_eq!(level.web.vertices[15], [1., 9.]);
    assert_eq!(level.web.neighbor(0, -1), None);
    assert_eq!(level.web.neighbor(14, 1), None);
    assert_eq!(
        level.web.orientations,
        [
            112, 64, 32, 0, -16, -48, -48, -64, -80, -96, -112, -128, 112, 112, 80
        ]
    );
    assert_eq!(level.flippers.count, 31);
    assert_eq!(level.flippers.first_spawn_ticks, 51);
    assert_eq!(level.flippers.spawn_interval_ticks, 101);
    assert_eq!(level.flippers.climb_speed, 0xa800);
    assert_eq!(level.enemy_shots, 3);
    assert!(level.tankers.is_none());
    let pulsars = level.pulsars.unwrap();
    assert_eq!(pulsars.generator.first_spawn_ticks, 251);
    assert_eq!(pulsars.generator.spawn_interval_ticks, 501);
    assert_eq!(pulsars.generator.climb_speed, 0x8000);
    assert_eq!(pulsars.phase_ticks, 9);
    assert_eq!(pulsars.propagation_ticks, 5);
    assert_eq!(level.spikers.unwrap().climb_speed, 0xd000);
    assert_eq!(level.beasts.unwrap().spawn_interval_ticks, 1001);
    assert_eq!(level.fuseballs.unwrap().spawn_interval_ticks, 901);
    assert_eq!(level.appearance.surface, [1., 0., 0.]);
    assert_eq!(level.appearance.flipper, [135, 208, 255]);
    assert_eq!(level.powerups.kill_interval, 7);
}

#[test]
fn original_level_seventeen_starts_the_red_oval_and_pulsar_wave() {
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/17.toml")),
        "level17",
    )
    .unwrap();
    assert_eq!(level.web.lanes(), 18);
    assert!(level.web.closed);
    assert_eq!(level.web.start_lane, 13);
    assert_eq!(level.web.vertices[0], [1., 8.]);
    assert_eq!(level.web.vertices[9], [15., 10.]);
    assert_eq!(
        level.web.orientations,
        [
            48, 16, 16, 0, 0, -16, -16, -48, -64, -80, -112, -112, -128, -128, 112, 112, 80, 64
        ]
    );
    assert_eq!(level.flippers.count, 25);
    assert_eq!(level.flippers.climb_speed, 0xa800);
    assert_eq!(level.flippers.spawn_interval_ticks, 121);
    assert_eq!(level.enemy_shots, 3);
    assert!(level.tankers.is_none());
    let pulsars = level.pulsars.unwrap();
    assert_eq!(pulsars.generator.first_spawn_ticks, 301);
    assert_eq!(pulsars.generator.spawn_interval_ticks, 601);
    assert_eq!(pulsars.generator.climb_speed, 0x8000);
    assert_eq!(pulsars.phase_ticks, 9);
    assert_eq!(pulsars.propagation_ticks, 5);
    assert_eq!(level.spikers.unwrap().climb_speed, 0xd000);
    assert_eq!(level.beasts.unwrap().spawn_interval_ticks, 1001);
    assert_eq!(level.fuseballs.unwrap().spawn_interval_ticks, 901);
    assert_eq!(level.appearance.surface, [1., 0., 0.]);
    assert_eq!(level.appearance.flipper, [135, 208, 255]);
    assert_eq!(level.powerups.kill_interval, 7);
    for bad in ["phase_ticks = 0", "propagation_ticks = 0"] {
        let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/17.toml"));
        let field = bad.split(" = ").next().unwrap();
        let valid = if field == "phase_ticks" {
            "phase_ticks = 9"
        } else {
            "propagation_ticks = 5"
        };
        assert!(
            Level::parse(&source.replace(valid, bad), "bad-pulsar")
                .err()
                .unwrap()
                .contains("must be positive")
        );
    }
}
use std::path::Path;

#[test]
fn original_level_five_uses_the_triangle_and_faster_climbers() {
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/05.toml")),
        "level5",
    )
    .unwrap();
    assert!(level.web.closed);
    assert_eq!(level.web.lanes(), 18);
    assert_eq!(level.web.start_lane, 9);
    assert_eq!(level.web.flip_angle(5, 6, 1), 48);
    assert_eq!(level.web.flip_angle(17, 0, 1), 32);
    assert_eq!(level.flippers.climb_speed, 0xa400);
    assert_eq!(level.tankers.as_ref().unwrap().climb_speed, 0x8800);
    assert_eq!(level.spikers.as_ref().unwrap().climb_speed, 0xe000);
    assert_eq!(level.powerups.kill_interval, 6);
}

#[test]
fn original_level_six_has_an_open_u_and_the_thirty_one_spawn_budget() {
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/06.toml")),
        "level6",
    )
    .unwrap();
    assert!(!level.web.closed);
    assert_eq!(level.web.lanes(), 15);
    assert_eq!(level.flippers.count, 31);
    assert_eq!(
        (
            level.flippers.first_spawn_ticks,
            level.flippers.spawn_interval_ticks
        ),
        (48, 96)
    );
    assert_eq!(level.tankers.as_ref().unwrap().spawn_interval_ticks, 261);
    assert_eq!(level.web.neighbor(14, 1), None);
    assert_eq!(level.web.neighbor(0, -1), None);
    assert_eq!(level.web.flip_angle(6, 7, 1), 128);
    assert_eq!(level.web.flip_angle(2, 3, 1), 112);
}

#[test]
fn original_level_seven_selects_the_t2k_lemon_and_its_speed_group() {
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/07.toml")),
        "level7",
    )
    .unwrap();
    assert!(level.web.closed);
    assert_eq!(level.web.lanes(), 14);
    assert_eq!(level.web.start_lane, 5);
    assert_eq!(level.web.vertices[0], [12., 3.]);
    assert_eq!(level.web.vertices[7], [3., 12.]);
    assert_eq!(level.flippers.climb_speed, 0xa600);
    assert_eq!(level.spikers.as_ref().unwrap().climb_speed, 0xf000);
    assert_eq!(
        (
            level.tankers.as_ref().unwrap().first_spawn_ticks,
            level.tankers.as_ref().unwrap().spawn_interval_ticks
        ),
        (126, 251)
    );
    assert_eq!(level.web.neighbor(13, 1), Some(0));
    assert_eq!(level.web.flip_angle(13, 0, 1), 64);
}

#[test]
fn original_level_eight_reuses_the_lemon_with_denser_tankers_and_spikers() {
    let seven = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/07.toml")),
        "level7",
    )
    .unwrap();
    let eight = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/08.toml")),
        "level8",
    )
    .unwrap();
    assert_eq!(seven.web.vertices, eight.web.vertices);
    assert_eq!(seven.web.orientations, eight.web.orientations);
    assert_eq!(eight.flippers.count, 31);
    assert_eq!(
        (
            eight.tankers.as_ref().unwrap().first_spawn_ticks,
            eight.tankers.as_ref().unwrap().spawn_interval_ticks
        ),
        (121, 241)
    );
    assert_eq!(
        (
            eight.spikers.as_ref().unwrap().first_spawn_ticks,
            eight.spikers.as_ref().unwrap().spawn_interval_ticks
        ),
        (326, 651)
    );
    assert_eq!(eight.tankers.as_ref().unwrap().super_chance, 28);
    assert!(
        Levels::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("levels"))
            .unwrap()
            .0
            .len()
            >= 8
    );
}

#[test]
fn original_level_nine_has_the_sine_wave_and_two_enemy_shots() {
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/09.toml")),
        "level9",
    )
    .unwrap();
    assert!(!level.web.closed);
    assert_eq!(level.web.lanes(), 14);
    assert_eq!(level.enemy_shots, 2);
    assert_eq!(level.flippers.climb_speed, 0xa800);
    assert_eq!(level.spikers.as_ref().unwrap().climb_speed, 0x10000);
    assert_eq!(level.tankers.as_ref().unwrap().spawn_interval_ticks, 231);
    assert_eq!(level.powerups.kill_interval, 7);
}

#[test]
fn original_level_ten_has_the_clover_and_beast_generator() {
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/10.toml")),
        "level10",
    )
    .unwrap();
    assert!(level.web.closed);
    assert_eq!(level.web.lanes(), 16);
    assert_eq!(level.web.vertices[0], [8., 6.]);
    assert_eq!(level.web.orientations[4], -16);
    assert_eq!(
        (
            level.beasts.as_ref().unwrap().first_spawn_ticks,
            level.beasts.as_ref().unwrap().spawn_interval_ticks
        ),
        (1001, 2001)
    );
}

#[test]
fn original_level_twelve_has_the_cross_and_faster_fuseball_generator() {
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/12.toml")),
        "level12",
    )
    .unwrap();
    assert!(level.web.closed);
    assert_eq!(level.web.lanes(), 16);
    assert_eq!(level.web.vertices[0], [7., 3.]);
    assert_eq!(level.flippers.climb_speed, 0xaa00);
    assert_eq!(level.fuseballs.as_ref().unwrap().spawn_interval_ticks, 701);
    assert_eq!(level.web.flip_angle(1, 2, 1), 160);
}

#[test]
fn original_level_thirteen_selects_the_t2k_giraffe_and_next_speed_group() {
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/13.toml")),
        "level13",
    )
    .unwrap();
    assert!(!level.web.closed);
    assert_eq!(level.web.lanes(), 13);
    assert_eq!(level.web.vertices[0], [14., -3.]);
    assert_eq!(level.flippers.climb_speed, 0xac00);
    assert_eq!(level.spikers.as_ref().unwrap().climb_speed, 0x12000);
    assert_eq!(level.fuseballs.as_ref().unwrap().climb_speed, 0x8800);
}

#[test]
fn original_level_fourteen_has_the_distorted_w_and_shorter_flipper_interval() {
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/14.toml")),
        "level14",
    )
    .unwrap();
    assert!(!level.web.closed);
    assert_eq!(level.web.lanes(), 15);
    assert_eq!(level.web.start_lane, 8);
    assert_eq!(level.flippers.spawn_interval_ticks, 76);
    assert_eq!(level.web.vertices[5], [13., 11.]);
    assert_eq!(level.tankers.as_ref().unwrap().super_chance, 52);
}

#[test]
fn original_level_fifteen_uses_web16_and_the_last_early_speed_group() {
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/15.toml")),
        "level15",
    )
    .unwrap();
    assert!(!level.web.closed);
    assert_eq!(level.web.lanes(), 15);
    assert_eq!(level.flippers.climb_speed, 0xae00);
    assert_eq!(level.spikers.as_ref().unwrap().climb_speed, 0x13000);
    assert_eq!(level.fuseballs.as_ref().unwrap().climb_speed, 0x9000);
    assert_eq!(level.fuseballs.as_ref().unwrap().spawn_interval_ticks, 501);
}

#[test]
fn original_level_sixteen_selects_the_compact_t2k_web_and_forty_one_spawns() {
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/16.toml")),
        "level16",
    )
    .unwrap();
    assert!(level.web.closed);
    assert_eq!(level.web.lanes(), 8);
    assert_eq!(level.flippers.count, 41);
    assert_eq!(level.flippers.spawn_interval_ticks, 71);
    assert_eq!(level.tankers.as_ref().unwrap().super_chance, 60);
    assert_eq!(level.enemy_shots, 2);
    assert_eq!(level.web.flip_angle(7, 0, 1), 96);
}

#[test]
fn open_plane_has_eleven_lanes_and_never_wraps_at_its_ends() {
    let level = Level::parse(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/03.toml")),
        "level3",
    )
    .unwrap();
    assert!(!level.web.closed);
    assert_eq!(level.web.lanes(), 11);
    assert_eq!(level.web.neighbor(0, -1), None);
    assert_eq!(level.web.neighbor(10, 1), None);
    assert_eq!(level.web.normalize(-LANE_UNIT), 0);
    assert_eq!(level.web.normalize(20 * LANE_UNIT), 10 * LANE_UNIT);
    assert_eq!(level.web.lane_delta(0, 10 * LANE_UNIT), 10 * LANE_UNIT);
    assert_eq!(level.web.vertex(11., FRONT_RADIUS, 0.).x, -22.0 / 3.0);
}

#[test]
fn original_levels_have_the_expected_geometry_and_wave_settings() {
    let levels = Levels::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("levels")).unwrap();
    assert!(levels.0.len() >= 2);
    let first = &levels.0[0];
    let second = &levels.0[1];
    assert_eq!(
        (
            first.flippers.count,
            first.flippers.first_spawn_ticks,
            first.flippers.spawn_interval_ticks
        ),
        (25, 61, 121)
    );
    assert_eq!(
        (
            second.flippers.count,
            second.flippers.first_spawn_ticks,
            second.flippers.spawn_interval_ticks
        ),
        (25, 51, 101)
    );
    assert_eq!(second.flippers.climb_speed, 0xa000);
    assert_eq!(second.web.lanes(), 16);
    assert_eq!(
        second.web.glider_transform(second.web.start()).translation,
        Vec3::new(-2.0 / 3.0, -8.0 / 3.0, 0.12)
    );
    for lane in 0..16 {
        let next = (lane + 1) % 16;
        assert_eq!(
            second.web.flip_angle(lane, next, 1),
            if lane % 4 == 3 { 64 } else { 128 }
        );
        assert_eq!(
            second.web.flip_angle(next, lane, -1),
            if lane % 4 == 3 { 64 } else { 128 }
        );
        assert_eq!(first.web.flip_angle(lane, next, 1), 112);
    }
}

#[test]
fn invalid_tweaks_report_the_file_and_reject_unknown_fields() {
    let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/01.toml"));
    for (old, new) in [
        ("start_lane = 7", "start_lane = 99"),
        ("depth = 160", "depth = 0"),
        ("first_spawn_ticks = 61", "first_spawn_ticks = 0"),
        ("climb_speed = 0.625", "climb_speed = nan"),
        ("flip_speed = 4", "flip_speed = 0"),
        ("kill_interval = 5", "kill_interval = 0"),
        ("spawn_interval_ticks = 121", "spawn_interva_ticks = 121"),
        ("[9, 2]", "[7, 2]"),
    ] {
        let error = Level::parse(&source.replace(old, new), "bad-level.toml")
            .err()
            .expect("invalid definition must fail");
        assert!(error.starts_with("bad-level.toml:"), "{error}");
    }
}
