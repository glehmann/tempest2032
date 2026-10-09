//! Tempest 2000 prototype entry point. Parses the command line and starts Bevy.

use clap::Parser;

mod animations;
mod app;
mod camera;
mod capture;
mod cli;
mod collisions;
mod cry;
mod enemies;
mod explosion;
mod frame_pacing;
mod game;
mod glider;
mod hud;
mod level;
mod mesh;
mod messages;
mod powerups;
mod scene;
mod sines;
mod stars;
mod warp;
mod weapons;
mod web;

#[cfg(test)]
mod test_support;

fn main() {
    app::run(cli::Cli::parse());
}
