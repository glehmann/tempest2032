//! Checks RGB channel decoding for the CRY hues used by power-ups and particle lasers.

use crate::cry::color;
use bevy::prelude::*;

#[test]
fn powerup_and_particle_laser_hues_use_all_three_channels() {
    assert_eq!(color(0x8a, 255), Color::srgb_u8(180, 254, 163));
    assert_eq!(color(0xaa, 255), Color::srgb_u8(226, 254, 140));
    assert_eq!(color(0x8a, 0), Color::srgb_u8(0, 0, 0));
}

#[test]
fn feedback_blends_rgb_between_exact_checkpoints_without_blending_cry_hues() {
    use bevy::{
        asset::RenderAssetUsages,
        render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    };

    let mut image = Image::new_fill(
        Extent3d::default(),
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    for phase in 0..=4 {
        crate::cry::write_feedback(&mut image, &[0x00ff], &[0xf0ff], phase);
        assert_eq!(
            image.data.as_ref().unwrap(),
            &[
                (254 * phase / 4) as u8,
                0,
                (254 * (4 - phase) / 4) as u8,
                255
            ]
        );
    }
}
