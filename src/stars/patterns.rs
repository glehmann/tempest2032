//! Original sine, menu, and ring field initializers from yak.s.

use super::{DEPTH_WRAP, Star};
use crate::{cry, sines::sine, web::LANE_UNIT};
use bevy::prelude::*;
use clap::ValueEnum;

// Even-offset bytes of src/dat/rantab.dat, masked to seven bits by siney (yak.s:9131).
const RANTAB: [u8; 256] = [
    92, 49, 117, 85, 44, 101, 66, 112, 40, 73, 14, 63, 2, 9, 61, 60, 42, 68, 88, 59, 86, 71, 36,
    122, 26, 90, 24, 111, 98, 43, 96, 105, 82, 18, 94, 110, 45, 68, 89, 46, 78, 55, 55, 114, 39,
    55, 120, 79, 61, 13, 123, 48, 54, 87, 7, 100, 49, 50, 17, 5, 95, 29, 30, 69, 30, 63, 112, 3, 1,
    28, 24, 106, 29, 32, 20, 98, 105, 77, 124, 11, 28, 45, 97, 39, 77, 0, 80, 41, 76, 1, 50, 74,
    65, 31, 126, 88, 15, 72, 91, 12, 13, 85, 73, 57, 27, 6, 5, 89, 127, 59, 54, 118, 38, 19, 106,
    86, 85, 123, 94, 24, 71, 113, 71, 79, 109, 10, 30, 8, 58, 7, 34, 1, 80, 83, 84, 7, 96, 29, 69,
    47, 96, 48, 65, 99, 55, 42, 31, 8, 105, 113, 103, 110, 95, 94, 15, 81, 25, 58, 62, 47, 73, 2,
    120, 0, 49, 1, 125, 9, 4, 54, 91, 88, 65, 116, 100, 11, 95, 31, 17, 97, 77, 66, 98, 80, 107,
    99, 105, 33, 4, 87, 107, 27, 117, 92, 126, 99, 39, 80, 64, 67, 40, 87, 52, 62, 53, 23, 110, 12,
    125, 81, 50, 116, 21, 48, 35, 103, 99, 12, 83, 75, 68, 51, 3, 21, 114, 15, 112, 14, 11, 94, 72,
    108, 125, 64, 124, 5, 50, 44, 103, 23, 15, 115, 84, 41, 26, 52, 64, 78, 101, 22, 89, 120, 2,
    57, 46, 127,
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub(crate) enum Pattern {
    #[default]
    Level,
    Rings,
    Selection,
}

impl Pattern {
    pub(super) fn stars(self, level: usize) -> Vec<Star> {
        match self {
            Self::Level => {
                let code = (level.saturating_sub(1) & 255) as u32;
                let code = if code == 31 { 163 } else { code };
                let x_step = (code + 1) * 2;
                let y_step = x_step + u32::from(RANTAB[x_step as usize / 2 % RANTAB.len()]) + 4;
                patterned(
                    x_step * LANE_UNIT as u32,
                    y_step * LANE_UNIT as u32,
                    0,
                    64 * LANE_UNIT as u32,
                )
            }
            Self::Selection => {
                let code = level.saturating_sub(1) as u32;
                let step = 0x60000_u32.wrapping_add(code.wrapping_mul(0x4000));
                patterned(step, step, 0, ((code & 127) + 64) * LANE_UNIT as u32)
            }
            Self::Rings => rings(200),
        }
    }

    pub(super) fn step(self) -> i32 {
        match self {
            Self::Rings => 0x1c000,
            Self::Selection => 6 * LANE_UNIT,
            _ => super::STEP,
        }
    }

    pub(super) fn trails(self, tick: u32) -> (u8, i32) {
        match self {
            Self::Rings => (6, ((tick % 1024) as i32 * 0x8000).rem_euclid(DEPTH_WRAP)),
            Self::Selection => (4, 26 * LANE_UNIT),
            _ => (0, 0),
        }
    }
}

fn patterned(x_step: u32, y_step: u32, mut x_phase: u32, mut y_phase: u32) -> Vec<Star> {
    (0..super::COUNT)
        .map(|index| {
            let x = sine((x_phase >> 16) as i32);
            let y = sine((y_phase >> 16) as i32);
            x_phase = x_phase.wrapping_add(x_step);
            y_phase = y_phase.wrapping_add(y_step);
            Star {
                position: Vec2::new(x as f32, -y as f32),
                depth: (255 - index as i32) * 2 * LANE_UNIT,
                color: cry::color((((x + 128) & 240) | ((y + 128) >> 4)) as u8, 255),
            }
        })
        .collect()
}

pub(super) fn rings(multiplier: i32) -> Vec<Star> {
    // DBRA runs 33 iterations per ring; the GPU reads only the first 256 entries.
    (0..8)
        .rev()
        .flat_map(|ring| {
            (0..=32).rev().map(move |step| {
                let angle = step * 8;
                let phase = ((angle + 64) & 255) * 4;
                let hue = ((7 - ring) * 0x20 + (((sine(phase) + 128) & 240) >> 4)) as u8;
                Star {
                    position: Vec2::new(sine(angle) as f32, -sine(angle + 64) as f32)
                        * (multiplier as f32 / 512.),
                    depth: ring * 64 * LANE_UNIT + sine(phase).abs() * LANE_UNIT / 2,
                    color: cry::color(hue, 255),
                }
            })
        })
        .take(super::COUNT)
        .collect()
}

#[derive(Resource, Default)]
pub(crate) struct Animation {
    pub(crate) pattern: Pattern,
    pub(super) level: Option<usize>,
    pub(super) offset: i32,
    pub(super) tick: u32,
    pub(super) epoch: Option<u32>,
    pub(super) selected: Option<Pattern>,
    pub(super) entry_motion: Vec<(i32, i32)>,
    pub(super) entry_warp: super::Warp,
}

impl Animation {
    pub(crate) fn new(pattern: Pattern) -> Self {
        Self {
            pattern,
            ..default()
        }
    }
}
