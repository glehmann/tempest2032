//! Original random, sine, menu, and ring field initializers from yak.s.

use super::{DEPTH_WRAP, Star};
use crate::{cry, web::LANE_UNIT};
use bevy::prelude::*;
use clap::ValueEnum;

const RANDOM: &[u8; 512] = include_bytes!("../../assets/stars/rantab.bin");
const SINES: &[u8; 256] = include_bytes!("../../assets/warp/sines.bin");

fn sine(angle: u32) -> i32 {
    i32::from(SINES[(angle & 255) as usize] as i8)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub(crate) enum Pattern {
    #[default]
    Level,
    Random,
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
                let y_step = x_step + u32::from(RANDOM[x_step as usize % RANDOM.len()] & 127) + 4;
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
            Self::Random => {
                let mut cursor = 0;
                let mut next = || {
                    cursor += 1;
                    RANDOM[cursor & 255]
                };
                (0..128)
                    .map(|_| {
                        let x = next();
                        let y = next();
                        let z = next();
                        Star {
                            position: Vec2::new(f32::from(x) - 128., 128. - f32::from(y)),
                            depth: i32::from(z) * 2 * LANE_UNIT,
                            color: cry::color((x & 240) | (y >> 4), 255),
                        }
                    })
                    .collect()
            }
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
            let x = sine(x_phase >> 16);
            let y = sine(y_phase >> 16);
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
}

impl Animation {
    pub(crate) fn new(pattern: Pattern) -> Self {
        Self {
            pattern,
            ..default()
        }
    }
}
