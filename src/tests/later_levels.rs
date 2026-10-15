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
fn level_23_uses_the_retail_d_shape_with_a_flat_base() {
    let level = level(23);
    assert!(level.web.closed);
    assert_eq!(level.web.lanes(), 15);
    assert_eq!(level.web.start_lane, 12);
    assert_eq!(level.web.vertices[0], [2., 12.]);
    assert_eq!(level.web.vertices[9], [14., 12.]);
    assert_eq!(&level.web.orientations[9..], &[-128; 6]);
    assert_eq!(level.flippers.climb_speed, 0xae00);
    assert_eq!(level.spikers.unwrap().climb_speed, 0xf000);
    assert_eq!(level.fuseballs.unwrap().climb_speed, 0xa800);
    assert_eq!(level.pulsars.unwrap().generator.spawn_interval_ticks, 463);
    assert_eq!(level.tankers.unwrap().super_chance, 88);
}

#[test]
fn level_24_has_the_retail_curved_l_and_shorter_generator_delays() {
    let level = level(24);
    assert!(!level.web.closed);
    assert_eq!(level.web.lanes(), 13);
    assert_eq!(level.web.start_lane, 4);
    assert_eq!(level.web.vertices[0], [15., 13.]);
    assert_eq!(level.web.vertices[13], [-2., 0.]);
    assert_eq!(level.tankers.unwrap().spawn_interval_ticks, 241);
    assert_eq!(level.spikers.unwrap().spawn_interval_ticks, 651);
    let pulsars = level.pulsars.unwrap();
    assert_eq!(pulsars.generator.first_spawn_ticks, 206);
    assert_eq!(pulsars.generator.spawn_interval_ticks, 412);
    assert_eq!(level.flippers.count, 36);
}

#[test]
fn level_25_introduces_fuseball_tankers_on_the_retail_double_curve() {
    let level = level(25);
    assert!(!level.web.closed);
    assert_eq!(level.web.lanes(), 12);
    assert_eq!(level.web.start_lane, 7);
    assert_eq!(level.web.vertices[0], [9., 16.]);
    assert_eq!(level.web.vertices[12], [-3., 4.]);
    assert_eq!(level.flippers.count, 41);
    assert_eq!(level.flippers.climb_speed, 0xb000);
    let tankers = level.fuse_tankers.unwrap();
    assert_eq!(tankers.first_spawn_ticks, 1001);
    assert_eq!(tankers.spawn_interval_ticks, 2001);
    assert_eq!(tankers.climb_speed, 0x9000);
    assert_eq!(level.pulsars.unwrap().phase_ticks, 7);
    assert_eq!(level.powerups.kill_interval, 8);
}

#[test]
fn fuseball_tankers_require_child_settings_with_either_line_ending() {
    let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/levels/25.toml"));
    let mut definition: toml::Table = toml::from_str(source).unwrap();
    assert!(definition.remove("fuseballs").is_some());
    let invalid = toml::to_string(&definition).unwrap();
    for invalid in [invalid.clone(), invalid.replace('\n', "\r\n")] {
        let error = Level::parse(&invalid, "invalid")
            .err()
            .expect("Fuseball Tankers without child settings must fail validation");
        assert!(error.contains("released children"), "{error}");
    }
}

#[test]
fn level_26_preserves_the_retail_cat_heads_inward_ear_corners() {
    let level = level(26);
    assert!(level.web.closed);
    assert_eq!(level.web.lanes(), 16);
    assert_eq!(level.web.start_lane, 8);
    assert_eq!(level.web.vertices[2], [12., 0.]);
    assert_eq!(level.web.vertices[14], [4., 0.]);
    assert_eq!(level.web.orientations[1].rem_euclid(256), 32);
    assert_eq!(level.web.orientations[2], -48);
    assert_eq!(level.pulsars.unwrap().generator.spawn_interval_ticks, 400);
    assert_eq!(level.spikers.unwrap().spawn_interval_ticks, 851);
    assert_eq!(level.fuse_tankers.unwrap().spawn_interval_ticks, 2001);
}

#[test]
fn level_27_keeps_the_retail_teardrops_tip_and_the_next_speed_group() {
    let level = level(27);
    assert!(level.web.closed);
    assert_eq!(level.web.lanes(), 15);
    assert_eq!(level.web.start_lane, 7);
    assert_eq!(level.web.vertices[0], [8., -1.]);
    assert_eq!(level.web.vertices[1], level.web.vertices[14]);
    assert_eq!(level.web.orientations[0], -64);
    assert_eq!(level.flippers.climb_speed, 0xb200);
    assert_eq!(level.spikers.unwrap().climb_speed, 0x11000);
    assert_eq!(level.fuseballs.unwrap().climb_speed, 0xb800);
    assert_eq!(level.pulsars.unwrap().generator.spawn_interval_ticks, 389);
    assert_eq!(level.tankers.unwrap().spawn_interval_ticks, 371);
}

#[test]
fn level_28_has_the_retail_open_crinkly_l() {
    let level = level(28);
    assert!(!level.web.closed);
    assert_eq!(level.web.lanes(), 14);
    assert_eq!(level.web.start_lane, 3);
    assert_eq!(level.web.vertices[0], [14., 16.]);
    assert_eq!(level.web.vertices[14], [1., -2.]);
    assert_eq!(&level.web.orientations[4..8], &[-176, -192, -176, -192]);
    assert_eq!(level.flippers.spawn_interval_ticks, 81);
    assert_eq!(level.pulsars.unwrap().generator.spawn_interval_ticks, 378);
    assert_eq!(level.fuseballs.unwrap().spawn_interval_ticks, 701);
    assert_eq!(level.tankers.unwrap().super_chance, 108);
}

#[test]
fn level_29_uses_the_retail_kiss_of_death_and_six_tick_pulsar_phases() {
    let level = level(29);
    assert!(level.web.closed);
    assert_eq!(level.web.lanes(), 13);
    assert_eq!(level.web.start_lane, 9);
    assert_eq!(level.web.vertices[0], [2., 7.]);
    assert_eq!(level.web.vertices[3], [8., 7.]);
    assert_eq!(&level.web.orientations[..6], &[-240, 0, -16, -240, 0, -16]);
    assert_eq!(level.flippers.climb_speed, 0xb400);
    assert_eq!(level.tankers.unwrap().climb_speed, 0x9800);
    assert_eq!(level.spikers.unwrap().climb_speed, 0x12000);
    assert_eq!(level.fuseballs.unwrap().climb_speed, 0xc000);
    let pulsars = level.pulsars.unwrap();
    assert_eq!(pulsars.phase_ticks, 6);
    assert_eq!(pulsars.generator.spawn_interval_ticks, 367);
}

#[test]
fn level_30_has_the_retail_backwards_c_and_more_frequent_fuse_tankers() {
    let level = level(30);
    assert!(!level.web.closed);
    assert_eq!(level.web.lanes(), 15);
    assert_eq!(level.web.start_lane, 12);
    assert_eq!(level.web.vertices[0], [6., 6.]);
    assert_eq!(level.web.vertices[15], [6., 10.]);
    assert_eq!(level.flippers.spawn_interval_ticks, 76);
    let tankers = level.fuse_tankers.unwrap();
    assert_eq!(tankers.first_spawn_ticks, 501);
    assert_eq!(tankers.spawn_interval_ticks, 1001);
    assert_eq!(level.pulsars.unwrap().generator.spawn_interval_ticks, 351);
}

#[test]
fn level_31_has_the_retail_retraced_star_and_the_last_second_set_speed_group() {
    let level = level(31);
    assert!(level.web.closed);
    assert_eq!(level.web.lanes(), 16);
    assert_eq!(level.web.start_lane, 9);
    assert_eq!(level.web.vertices[0], [8., 3.]);
    assert_eq!(level.web.vertices[3], level.web.vertices[5]);
    assert_eq!(level.web.vertices[7], level.web.vertices[9]);
    assert_eq!(level.web.orientations[3], 0);
    assert_eq!(level.web.orientations[4], -128);
    assert_eq!(level.flippers.climb_speed, 0xb600);
    assert_eq!(level.flippers.spawn_interval_ticks, 96);
    assert_eq!(level.spikers.unwrap().climb_speed, 0x13000);
    assert_eq!(level.fuseballs.unwrap().climb_speed, 0xc800);
    assert_eq!(level.tankers.unwrap().super_chance, 120);
}

#[test]
fn level_32_finishes_the_second_web_set_with_the_retail_mouse_pointer() {
    let level = level(32);
    assert!(!level.web.closed);
    assert_eq!(level.web.lanes(), 17);
    assert_eq!(level.web.start_lane, 13);
    assert_eq!(level.web.vertices[0], [-1., 13.]);
    assert_eq!(level.web.vertices[17], [2., 16.]);
    assert_eq!(level.web.neighbor(0, -1), None);
    assert_eq!(level.web.neighbor(16, 1), None);
    assert_eq!(level.flippers.count, 41);
    assert_eq!(level.tankers.unwrap().spawn_interval_ticks, 431);
    assert_eq!(level.pulsars.unwrap().generator.spawn_interval_ticks, 901);
    assert_eq!(level.fuseballs.unwrap().spawn_interval_ticks, 501);
    assert_eq!(level.fuse_tankers.unwrap().spawn_interval_ticks, 501);
    assert_eq!(level.enemy_shots, 4);
}

#[test]
fn mirrored_lane_rotations_point_along_the_actual_web_edges() {
    use crate::web::FRONT_RADIUS;
    use bevy::prelude::*;
    let levels = Levels::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("levels")).unwrap();
    for level in &levels.0[17..] {
        for lane in 0..level.web.lanes() {
            let tangent = (level.web.vertex(lane as f32 + 1., FRONT_RADIUS, 0.)
                - level.web.vertex(lane as f32, FRONT_RADIUS, 0.))
            .normalize();
            assert!(
                (level.web.orientation(lane) * Vec3::X).dot(tangent) > 0.5,
                "level {}, lane {lane} has a reversed or misaligned orientation",
                level.number
            );
        }
    }
}
