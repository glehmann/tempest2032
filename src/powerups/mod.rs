//! Power-up reward progression and persistent upgrade state. Pickups award
//! particle laser, points, jumping, an AI droid, and superzap warp tokens.

use crate::powerups::droid::droid_mesh;
use bevy::prelude::*;

pub(crate) mod droid;
pub(crate) mod jump;
pub(crate) mod pickups;
pub(crate) mod render;

pub(crate) use droid::{Droid, move_droid, spawn_droid};
pub(crate) use jump::{queue_jump, update_jump};
pub(crate) use pickups::{PowerUp, advance_powerups, spawn_drop};
pub(crate) use render::draw_powerups;

#[derive(Resource)]
pub(crate) struct Upgrades {
    delay: u8,
    stage: u8,
    pub(crate) laser: bool,
    pub(crate) jump_enabled: bool,
    pub(crate) jump_height: i32,
    jump_velocity: i32,
    jump_queued: bool,
    pub(crate) droid: bool,
    warp_tokens: u8,
    frame: u32,
    notice: &'static str,
    notice_ticks: u16,
    random: u32,
}

impl Default for Upgrades {
    fn default() -> Self {
        Self {
            delay: 0,
            stage: 0,
            laser: false,
            jump_enabled: false,
            jump_height: 0,
            jump_velocity: 0,
            jump_queued: false,
            droid: false,
            warp_tokens: 0,
            frame: 0,
            notice: "",
            notice_ticks: 0,
            random: 0x2000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reward {
    Laser,
    Bonus,
    Jump,
    Droid,
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

    fn message(&mut self, message: &'static str) {
        self.notice = message;
        self.notice_ticks = 120;
    }

    fn collect(&mut self, score: &mut u32) -> Reward {
        let reward = match self.stage {
            0 => {
                self.laser = true;
                self.message("PARTICLE LASER!");
                Reward::Laser
            }
            2 => {
                self.jump_enabled = true;
                self.message("JUMP ENABLED!  J / SHIFT to jump");
                Reward::Jump
            }
            4 if !self.droid => {
                self.droid = true;
                self.message("A.I. DROID!");
                Reward::Droid
            }
            5 => {
                self.warp_tokens = (self.warp_tokens + 1).min(3);
                self.message(match self.warp_tokens {
                    1 => "SUPERZAP!  2 warp tokens to go",
                    2 => "SUPERZAP!  1 warp token to go",
                    _ => "SUPERZAP!  WARP READY!",
                });
                Reward::Zap
            }
            _ => {
                self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
                if self.random >> 24 <= 8 {
                    *score += 3000;
                    self.message("OUTTAH HERE!  +3000");
                    Reward::Skip
                } else {
                    *score += 2000;
                    self.message("2000 POINTS!");
                    Reward::Bonus
                }
            }
        };
        self.stage = (self.stage + 1).min(7);
        reward
    }

    pub(crate) fn lose_life(&mut self) {
        let tokens = self.warp_tokens;
        *self = Self::default();
        self.warp_tokens = tokens;
    }

    pub(crate) fn land(&mut self) {
        self.jump_height = 0;
        self.jump_velocity = 0;
        self.jump_queued = false;
    }

    pub(crate) fn begin_level(&mut self) {
        let (tokens, random, frame) = (self.warp_tokens, self.random, self.frame);
        *self = Self::default();
        self.warp_tokens = tokens;
        self.random = random;
        self.frame = frame;
    }

    pub(crate) fn warp_tokens(&self) -> u8 {
        self.warp_tokens
    }

    pub(crate) fn take_warp_tokens(&mut self) -> bool {
        if self.warp_tokens < 3 {
            return false;
        }
        self.warp_tokens = 0;
        true
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

#[derive(Component)]
pub(crate) struct PowerHud;

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
    commands.spawn((
        PowerHud,
        Visibility::Visible,
        Text::default(),
        TextFont {
            font_size: 25.0,
            ..default()
        },
        TextColor(Color::srgb(0.4, 0.9, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: px(100),
            left: px(26),
            ..default()
        },
    ));
}

pub(crate) fn update_notice(
    upgrades: Res<Upgrades>,
    game: Res<crate::game::Game>,
    mut hud: Single<(&mut Text, &mut Visibility), With<PowerHud>>,
) {
    if game.status == crate::game::Status::Bonus {
        *hud.1 = Visibility::Hidden;
        return;
    }
    *hud.1 = Visibility::Visible;
    let active = format!(
        "{}{}{}",
        if upgrades.laser { "LASER  " } else { "" },
        if upgrades.jump_enabled {
            "JUMP [J / SHIFT]  "
        } else {
            ""
        },
        if upgrades.droid { "DROID" } else { "" }
    );
    let notice = if upgrades.notice_ticks > 0 {
        upgrades.notice
    } else {
        ""
    };
    let text = format!(
        "{active}\nWARP TOKENS {}/3\n{notice}",
        upgrades.warp_tokens()
    );
    if hud.0.0 != text {
        hud.0.0 = text;
    }
}

#[cfg(test)]
#[path = "tests/powerups.rs"]
mod tests;
