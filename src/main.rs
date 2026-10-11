//! Parse the command line, install a Steam shortcut, or start the Bevy game.

use clap::Parser;

mod animations;
mod app;
mod camera;
mod capture;
mod cli;
mod collisions;
mod cry;
mod difficulty;
mod enemies;
mod explosion;
mod frame_pacing;
mod game;
mod gamepad;
mod glider;
mod hud;
mod level;
mod mesh;
mod messages;
mod music;
mod pause;
mod powerups;
mod scene;
mod sines;
mod sound;
mod stars;
mod steam;
mod warp;
mod weapons;
mod web;

#[cfg(test)]
mod test_support;

fn main() {
    let args = cli::Cli::parse();
    if args.steam_install {
        if let Err(error) = steam::run(args.steam_dir.as_deref(), args.steam_user) {
            eprintln!("Steam installation failed: {error}");
            std::process::exit(1);
        }
    } else {
        app::run(args);
    }
}
