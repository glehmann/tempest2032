//! Checks RGB channel decoding for the CRY hues used by power-ups and particle lasers.

use crate::cry::color;
use bevy::prelude::*;

#[test]
fn powerup_and_particle_laser_hues_use_all_three_channels() {
    assert_eq!(color(0x8a, 255), Color::srgb_u8(180, 254, 163));
    assert_eq!(color(0xaa, 255), Color::srgb_u8(226, 254, 140));
    assert_eq!(color(0x8a, 0), Color::srgb_u8(0, 0, 0));
}
