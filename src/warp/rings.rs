//! Original pring/psphere particles and CRY scale-and-rotate feedback.

use super::{Gate, Phase, UNIT, sine};
use bevy::prelude::*;
use std::collections::VecDeque;

use crate::cry::{FEEDBACK_HEIGHT as HEIGHT, FEEDBACK_WIDTH as WIDTH};

#[derive(Clone, Debug)]
pub(crate) struct Rings {
    pub(super) pixels: Vec<u16>,
    scratch: Vec<u16>,
    pub(super) rotation: i32,
    pub(super) fade: u32,
    passes: u32,
}

impl Default for Rings {
    fn default() -> Self {
        Self {
            pixels: vec![0; WIDTH * HEIGHT],
            scratch: vec![0; WIDTH * HEIGHT],
            rotation: 3,
            fade: 0xff0000,
            passes: 3,
        }
    }
}

fn pulse(angle: i32) -> u8 {
    (((sine(angle) + 128) & 0xf0) | ((sine(angle + 64) + 128) >> 4)) as u8
}

impl Rings {
    pub(super) fn pass(&mut self) {
        self.passes += 1;
        self.rotation = (self.passes & 7) as i32 - 3;
        self.fade = 0x1c0000;
    }

    pub(super) fn feedback(&mut self, angle: i32, scale: Vec2, fade: i32) {
        crate::cry::feedback(
            &mut self.pixels,
            &mut self.scratch,
            UVec2::new(WIDTH as u32, HEIGHT as u32),
            angle,
            scale,
            fade,
        );
    }

    fn plot(&mut self, point: Vec2, color: u16, size: i32) {
        let x = point.x as i32 + 192;
        let y = point.y as i32 + 120;
        for dy in 0..size {
            for dx in 0..size {
                if (0..WIDTH as i32).contains(&(x + dx)) && (0..HEIGHT as i32).contains(&(y + dy)) {
                    self.pixels[(y + dy) as usize * WIDTH + (x + dx) as usize] = color;
                }
            }
        }
    }

    fn sphere(&mut self, tick: u32) {
        let pulse_size = ((tick & 127) as i32 - 63).abs();
        let radius = pulse_size + 4;
        let hue = u16::from(pulse(tick as i32) & 0xf0) << 8;
        for ring in 0..16 {
            let twist = tick as i32 + ring * 8;
            for point in 0..128 {
                // psphere advances the XY phase twice, for four-angle-unit steps.
                let angle = point * 4;
                let x = sine(angle) * radius * sine(twist + 64);
                let y = sine(angle + 64) * radius * 128;
                let z = 64 * UNIT - sine(angle) * radius * sine(twist);
                if z < 0 {
                    continue;
                }
                let depth = (z >> 14) + 1;
                let position = Vec2::new(((x / depth) >> 7) as f32, ((y / depth) >> 7) as f32);
                let intensity = ((255 - ((depth >> 4) & 255)) * pulse_size) >> 8;
                self.plot(position, hue | intensity as u16, 1);
            }
        }
    }

    fn triangle(&mut self, vertices: [Vec2; 3], hue: u8) {
        let cross = |a: Vec2, b: Vec2| a.x * b.y - a.y * b.x;
        let area = cross(vertices[1] - vertices[0], vertices[2] - vertices[0]);
        if area.abs() < 0.01 {
            return;
        }
        let min = vertices
            .iter()
            .copied()
            .fold(Vec2::splat(f32::INFINITY), Vec2::min)
            .floor()
            .max(Vec2::new(-192., -120.));
        let max = vertices
            .iter()
            .copied()
            .fold(Vec2::splat(f32::NEG_INFINITY), Vec2::max)
            .ceil()
            .min(Vec2::new(191., 119.));
        for y in min.y as i32..=max.y as i32 {
            for x in min.x as i32..=max.x as i32 {
                let p = Vec2::new(x as f32, y as f32);
                let a = cross(vertices[1] - p, vertices[2] - p) / area;
                let b = cross(vertices[2] - p, vertices[0] - p) / area;
                let c = 1. - a - b;
                if a >= 0. && b >= 0. && c >= 0. {
                    self.plot(
                        p,
                        u16::from(hue) << 8 | (a * 255. + b * 192. + c * 128.) as u16,
                        1,
                    );
                }
            }
        }
    }

    fn icon(&mut self, center: Vec2, z: f32, kind: u8) {
        let (triangles, hue): (&[[Vec2; 3]], u8) = match kind {
            1 | 2 => (
                &[
                    [Vec2::new(-4., 2.), Vec2::new(0., 6.), Vec2::new(4., 2.)],
                    [Vec2::new(-2., 2.), Vec2::new(2., 2.), Vec2::new(-2., -4.)],
                    [Vec2::new(2., 2.), Vec2::new(-2., -4.), Vec2::new(2., -4.)],
                ],
                0xef,
            ),
            3 => (
                &[
                    [Vec2::new(0., 4.), Vec2::new(-4., 0.), Vec2::new(-2., 0.)],
                    [Vec2::new(0., 4.), Vec2::new(2., 0.), Vec2::new(4., 0.)],
                    [Vec2::ZERO, Vec2::new(-4., -4.), Vec2::new(-2., -4.)],
                    [Vec2::ZERO, Vec2::new(2., -4.), Vec2::new(4., -4.)],
                ],
                0x36,
            ),
            4 => (
                &[
                    [Vec2::new(0., 8.), Vec2::new(8., -8.), Vec2::new(0., -4.)],
                    [Vec2::new(8., -8.), Vec2::new(-8., -8.), Vec2::new(0., -4.)],
                    [Vec2::new(-8., -8.), Vec2::new(0., 8.), Vec2::new(0., -4.)],
                ],
                0xfc,
            ),
            _ => return,
        };
        for triangle in triangles {
            self.triangle(
                triangle.map(|p| {
                    center + Vec2::new(p.x, -p.y) * (64. / z) * if kind == 2 { -1. } else { 1. }
                }),
                hue,
            );
        }
    }

    pub(super) fn advance(
        &mut self,
        tick: u32,
        position: Vec2,
        gates: &VecDeque<Gate>,
        specs: &[(i8, i8, u8)],
        phase: Phase,
    ) {
        if !matches!(phase, Phase::Flying) {
            let age = match phase {
                Phase::Failure(ticks) | Phase::Success(ticks) => 151 - ticks.min(151),
                _ => return,
            };
            if !age.is_multiple_of(4) {
                return;
            }
            self.feedback(
                if matches!(phase, Phase::Failure(_)) {
                    1
                } else {
                    0
                },
                Vec2::splat(500. / 256.),
                -8 * UNIT,
            );
            return;
        }
        if self.fade >> 16 != 0xf0 {
            self.fade = self.fade.wrapping_sub(0xc000) & 0xffffff;
        }
        let fade = (self.fade << 8) as i32 >> 8;
        self.feedback(self.rotation, Vec2::new(468. / 256., 484. / 256.), fade);
        self.sphere(tick);
        for (gate, (x, y, kind)) in gates.iter().zip(specs) {
            let z = gate.distance as f32 / UNIT as f32;
            if z <= 0. {
                continue;
            }
            let center = (Vec2::new(f32::from(*x), f32::from(*y)) - position) * (128. / z);
            // pring's radius 19 is multiplied by signed-byte sine and shifted by 8.
            let radius = 19. * 128. / z;
            let color = u16::from(pulse(tick as i32 * 2) & 15) << 8
                | ((1300. - z * 4.).max(0.) * 255. / 1300.) as u16;
            self.icon(center, z, *kind);
            for point in 0..=128 {
                let angle = point * 2;
                self.plot(
                    center
                        + Vec2::new(sine(angle) as f32, sine(angle + 64) as f32) * (radius / 256.),
                    color,
                    2,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_brighten_feedback_and_cycle_rotation_without_reinitializing_the_image() {
        let mut rings = Rings::default();
        rings.pixels[120 * WIDTH + 192] = 0x8fff;
        rings.pass();
        assert_eq!((rings.fade, rings.rotation), (0x1c0000, 1));
        assert_eq!(rings.pixels[120 * WIDTH + 192], 0x8fff);
        for expected in [2, 3, 4, -3, -2, -1, 0, 1] {
            rings.pass();
            assert_eq!(rings.rotation, expected);
        }
    }

    #[test]
    fn feedback_fades_preserves_cry_hues_and_injects_new_particles() {
        let mut rings = Rings::default();
        rings.pixels[120 * WIDTH + 192] = 0x8fff;
        rings.feedback(0, Vec2::splat(2.), -8 * UNIT);
        assert_eq!(rings.pixels[120 * WIDTH + 192], 0x8ff7);
        rings.advance(
            60,
            Vec2::ZERO,
            &VecDeque::from([Gate {
                index: 0,
                distance: 100 * UNIT,
                age: 0,
            }]),
            &[(0, 0, 0)],
            Phase::Flying,
        );
        assert!(rings.pixels.iter().filter(|p| **p & 255 != 0).count() > 128);
        assert!(rings.pixels.iter().any(|p| *p >> 8 != 0));
    }
}
