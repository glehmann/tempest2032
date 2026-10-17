//! Power-up reward progression and persistent upgrade state. Pickups award
//! particle laser, points, jumping, an AI droid, and superzap warp tokens.

use crate::game::{Game, Status};
use crate::powerups::droid::droid_mesh;
use bevy::prelude::*;

pub(crate) mod droid;
pub(crate) mod jump;
pub(crate) mod pickups;
pub(crate) mod render;
pub(crate) mod superzapper;

pub(crate) use droid::{Droid, move_droid, spawn_droid};
pub(crate) use jump::{queue_jump, update_jump};
pub(crate) use pickups::{advance_powerups, spawn_drop};
pub(crate) use render::draw_powerups;

#[derive(Resource, Default)]
pub(crate) struct Upgrades {
    delay: u8,
    stage: u8,
    pub(crate) laser: bool,
    pub(crate) jump_enabled: bool,
    pub(crate) jump_height: i32,
    jump_velocity: i32,
    pub(crate) jump_queued: bool,
    pub(crate) zapper: superzapper::Superzapper,
    pub(crate) droid: bool,
    droid_next: bool,
    warp_tokens: u8,
    frame: u32,
    pub(crate) announcements: Vec<crate::messages::Id>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reward {
    Laser,
    Bonus,
    Jump,
    Droid,
    DroidNext,
    Zap,
    Skip,
}

impl Upgrades {
    pub(crate) fn drop_due(&mut self, interval: u8) -> bool {
        if self.delay == 0 {
            self.delay = interval - 1;
            true
        } else {
            self.delay -= 1;
            false
        }
    }

    fn message(&mut self, message: crate::messages::Id) {
        self.announcements.push(message);
    }

    fn collect(&mut self, game: &mut Game) -> Reward {
        if game.status == Status::Exiting {
            if !self.droid_next {
                self.droid_next = true;
                self.message(crate::messages::Id::YesYesYes);
                return Reward::DroidNext;
            }
            self.stage = 1;
        } else if self.droid_next {
            self.droid_next = false;
            self.droid = true;
            self.message(crate::messages::Id::AiDroid);
            return Reward::Droid;
        }
        // An early droid leaves bonum unchanged; its normal slot becomes superzap.
        if self.stage == 4 && self.droid {
            self.stage = 5;
        }
        let reward = match self.stage {
            0 => {
                self.laser = true;
                self.message(crate::messages::Id::ParticleLaser);
                Reward::Laser
            }
            2 => {
                self.jump_enabled = true;
                self.message(crate::messages::Id::JumpEnabled);
                Reward::Jump
            }
            4 => {
                self.droid = true;
                self.message(crate::messages::Id::AiDroid);
                Reward::Droid
            }
            5 => {
                self.zapper.powerup();
                self.message(crate::messages::Id::Excellent);
                self.warp_tokens = (self.warp_tokens + 1).min(3);
                self.message(match self.warp_tokens {
                    1 => crate::messages::Id::WarpTwo,
                    2 => crate::messages::Id::WarpOne,
                    _ => crate::messages::Id::WarpEnabled,
                });
                Reward::Zap
            }
            _ => {
                // suprise uses rannum's shared gameplay stream, not a pickup-only sequence.
                game.advance_random();
                if game.random >> 24 <= 8 {
                    game.add_score(3000);
                    self.message(crate::messages::Id::OuttaHere);
                    self.message(crate::messages::Id::Excellent);
                    Reward::Skip
                } else {
                    game.add_score(2000);
                    self.message(match self.stage {
                        1 | 6 => crate::messages::Id::Zappo,
                        3 => crate::messages::Id::Outstanding,
                        _ => crate::messages::Id::Dude,
                    });
                    self.message(crate::messages::Id::Points2000);
                    Reward::Bonus
                }
            }
        };
        self.stage = (self.stage + 1).min(7);
        reward
    }

    pub(crate) fn lose_life(&mut self) {
        let (tokens, droid_next) = (self.warp_tokens, self.droid_next);
        *self = Self::default();
        self.warp_tokens = tokens;
        self.droid_next = droid_next;
    }

    pub(crate) fn land(&mut self) {
        self.zapper.cancel();
        self.jump_height = 0;
        self.jump_velocity = 0;
        self.jump_queued = false;
    }

    pub(crate) fn begin_level(&mut self) {
        let (tokens, frame, droid_next) = (self.warp_tokens, self.frame, self.droid_next);
        *self = Self::default();
        self.warp_tokens = tokens;
        self.frame = frame;
        self.droid_next = droid_next;
    }

    pub(crate) fn warp_tokens(&self) -> u8 {
        self.warp_tokens
    }

    pub(crate) fn take_warp_tokens(&mut self) {
        self.warp_tokens = 0;
    }

    #[cfg(test)]
    pub(crate) fn seed_warp_tokens(&mut self, tokens: u8) {
        self.warp_tokens = tokens.min(3);
    }
}

#[derive(Resource, Default)]
pub(crate) struct PowerAssets {
    material: Handle<StandardMaterial>,
    droid: Handle<Mesh>,
    droid_material: Handle<StandardMaterial>,
}

pub(crate) fn setup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.insert_resource(PowerAssets {
        material: materials.add(StandardMaterial {
            unlit: true,
            cull_mode: None,
            ..default()
        }),
        droid: meshes.add(droid_mesh()),
        droid_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.85, 1.0),
            unlit: true,
            cull_mode: None,
            ..default()
        }),
    });
}

#[cfg(test)]
#[path = "tests/powerups.rs"]
mod tests;
