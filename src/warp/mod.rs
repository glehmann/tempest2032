//! Warp-zone selection, entry/return transactions, and fixed-tick gate flight.

use crate::{
    game::{Game, Status},
    level::{Levels, LoadReason},
};
use bevy::prelude::*;
use clap::ValueEnum;
use std::collections::VecDeque;

mod courses;
pub(crate) mod render;

const UNIT: i32 = 1 << 16;
pub(crate) const SCALE: f32 = 0.1;
pub(crate) const GATES: usize = 53;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(crate) enum ZoneKind {
    #[value(alias = "gate", alias = "1")]
    GateFlight,
    #[value(skip)]
    Tunnel,
    #[value(skip)]
    Rings,
}

impl ZoneKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::GateFlight => "GATE FLIGHT",
            Self::Tunnel => "GREEN TRACK",
            Self::Rings => "PSYCHEDELIC RINGS",
        }
    }
    pub(crate) fn index(self) -> usize {
        match self {
            Self::GateFlight => 0,
            Self::Tunnel => 1,
            Self::Rings => 2,
        }
    }
    pub(crate) fn for_web(number: usize) -> Self {
        match ((number - 1) / 16) & 3 {
            0 => Self::GateFlight,
            1 | 3 => Self::Tunnel,
            _ => Self::Rings,
        }
    }
    pub(crate) fn available(self) -> bool {
        self == Self::GateFlight
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Phase {
    Flying,
    Failure(u16),
    Success(u16),
    Victory(u16),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Gate {
    pub(crate) index: usize,
    pub(crate) distance: i32,
    pub(crate) age: u16,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Axis {
    pub(crate) position: i32,
    pub(crate) velocity: i32,
}

impl Axis {
    fn step(
        &mut self,
        direction: i32,
        acceleration: i32,
        friction: i32,
        max_velocity: i32,
        min: i32,
        max: i32,
    ) {
        if direction == 0 {
            if self.velocity.abs() < friction {
                self.velocity = 0;
                return;
            }
            self.velocity -= self.velocity.signum() * friction;
        } else {
            if (direction < 0 && self.position < min) || (direction > 0 && self.position >= max) {
                self.velocity = 0;
                return;
            }
            self.velocity =
                (self.velocity + direction * acceleration).clamp(-max_velocity, max_velocity);
        }
        self.position += self.velocity;
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Session {
    pub(crate) kind: ZoneKind,
    pub(crate) course: usize,
    pub(crate) destination: usize,
    pub(crate) phase: Phase,
    pub(crate) x: Axis,
    pub(crate) y: Axis,
    pub(crate) speed: i32,
    pub(crate) distance: i64,
    pub(crate) gates: VecDeque<Gate>,
    pub(crate) passed: usize,
    pub(crate) tick: u32,
    pub(crate) notice: &'static str,
    notice_ticks: u16,
    cursor: usize,
    generation_distance: i32,
    generation_wait: i16,
}

impl Session {
    pub(crate) fn new(
        kind: ZoneKind,
        attempt: u32,
        course: Option<u8>,
        destination: usize,
    ) -> Self {
        Self {
            kind,
            course: course.map_or(courses::ORDER[attempt as usize & 7], |n| usize::from(n - 1)),
            destination,
            phase: Phase::Flying,
            x: Axis {
                position: 160 * UNIT,
                velocity: 0,
            },
            y: Axis {
                position: 8 * UNIT,
                velocity: 0x80000,
            },
            speed: 0x8000 + ((attempt.wrapping_mul(0x200) & 0xffff) as i32),
            distance: 0,
            gates: VecDeque::new(),
            passed: 0,
            tick: 0,
            notice: "FLY THROUGH EVERY GATE",
            notice_ticks: 180,
            cursor: 0,
            generation_distance: 0,
            generation_wait: 0,
        }
    }
    pub(crate) fn position(&self) -> Vec2 {
        Vec2::new(
            (self.x.position - 160 * UNIT) as f32 / UNIT as f32,
            self.y.position as f32 / (8 * UNIT) as f32,
        )
    }
    pub(crate) fn spec(&self, index: usize) -> (i8, i8, u8) {
        let record = courses::COURSES[self.course][index];
        (
            (record >> 16) as u8 as i8,
            (record >> 8) as u8 as i8,
            record as u8,
        )
    }
    fn pass(&mut self, index: usize) -> Option<u32> {
        let (x, y, kind) = self.spec(index);
        let dx = ((self.x.position - 160 * UNIT) >> 16) - i32::from(x);
        let dy = (self.y.position >> 19) - i32::from(y);
        if dx.abs() >= 8 || dy.abs() >= 8 {
            self.phase = Phase::Failure(151);
            self.notice = "WARP FAILED";
            return None;
        }
        let error = dx.abs() + dy.abs();
        self.passed += 1;
        self.notice = match kind {
            1 => {
                self.y.velocity = -0x80000;
                "UP BOOST"
            }
            2 => {
                self.y.velocity = 0x80000;
                "DOWN BOOST"
            }
            3 => {
                self.speed += self.speed >> 3;
                "SPEED BOOST"
            }
            4 => {
                self.phase = Phase::Success(151);
                "COURSE COMPLETE"
            }
            _ => "GATE PASSED",
        };
        self.notice_ticks = 120;
        Some(if error < 4 {
            750
        } else if error < 8 {
            500
        } else {
            250
        })
    }
    fn step_flight(&mut self, horizontal: i32, vertical: i32) -> u32 {
        self.notice_ticks = self.notice_ticks.saturating_sub(1);
        if self.notice_ticks == 0 {
            self.notice = "";
        }
        self.x
            .step(horizontal, 0x1000, 0x800, 0x20000, 0, 320 * UNIT);
        self.y
            .step(vertical, 0x8000, 0x2000, 0x80000, -320 * UNIT, 320 * UNIT);
        self.distance += i64::from(self.speed);
        for gate in &mut self.gates {
            gate.distance -= self.speed;
            gate.age = gate.age.wrapping_add(1);
        }
        let mut points = 0;
        while self.gates.front().is_some_and(|g| g.distance >> 16 <= 2) {
            let gate = self.gates.pop_front().unwrap();
            if let Some(score) = self.pass(gate.index) {
                points += score;
            }
            if !matches!(self.phase, Phase::Flying) {
                return points;
            }
        }
        self.generation_distance -= self.speed;
        if self.generation_distance < 0 {
            self.generation_distance += 12 * UNIT;
            self.generation_wait -= 1;
            if self.generation_wait < 0 && self.cursor < GATES {
                self.generation_wait = (courses::COURSES[self.course][self.cursor] >> 24) as i16;
                self.gates.push_back(Gate {
                    index: self.cursor,
                    distance: 255 * UNIT,
                    age: 0,
                });
                self.cursor += 1;
            }
        }
        points
    }
}

pub(crate) fn advance(
    keys: Res<ButtonInput<KeyCode>>,
    levels: Res<Levels>,
    mut game: ResMut<Game>,
) {
    if game.status != Status::Bonus || game.load.is_some() {
        return;
    }
    let Some(mut session) = game.bonus.take() else {
        return;
    };
    session.tick = session.tick.wrapping_add(1);
    match session.phase {
        Phase::Flying => {
            let horizontal = i32::from(keys.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]))
                - i32::from(keys.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]));
            let vertical = i32::from(keys.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]))
                - i32::from(keys.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]));
            game.score += session.step_flight(horizontal, vertical);
        }
        Phase::Failure(1) | Phase::Victory(1) => {
            game.load = Some(LoadReason::ReturnWarp {
                destination: session.destination,
            });
        }
        Phase::Failure(ticks) => session.phase = Phase::Failure(ticks - 1),
        Phase::Success(1) => {
            game.score += 20_000;
            if levels.0[session.destination].number < 91 {
                session.destination = (session.destination + 4) % levels.0.len();
                session.notice = "EXCELLENT!  +20000  WARP FIVE LEVELS";
            } else {
                session.notice = "EXCELLENT!  +20000";
            }
            session.phase = Phase::Victory(401);
        }
        Phase::Success(ticks) => session.phase = Phase::Success(ticks - 1),
        Phase::Victory(ticks) => session.phase = Phase::Victory(ticks - 1),
    }
    game.bonus = Some(session);
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
