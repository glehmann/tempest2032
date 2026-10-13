//! Parse the command line, install a Steam shortcut, or start the Bevy game.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;

#[cfg(unix)]
#[global_allocator]
static ALLOCATOR: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod animations;
mod app;
#[cfg(target_arch = "wasm32")]
mod browser;
mod camera;
#[cfg(not(target_arch = "wasm32"))]
mod capture;
mod cli;
mod collisions;
mod cry;
mod difficulty;
mod enemies;
mod explosion;
#[cfg(not(target_arch = "wasm32"))]
mod frame_pacing;
mod game;
mod gamepad;
mod glider;
mod hud;
mod level;
mod menu;
mod mesh;
mod messages;
mod music;
mod pause;
mod powerups;
mod scene;
mod scores;
mod sines;
mod sound;
mod stars;
#[cfg(not(target_arch = "wasm32"))]
mod steam;
mod warp;
mod weapons;
mod web;

#[cfg(test)]
mod test_support;

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(target_arch = "wasm32")]
fn main() {
    app::run(browser::options());
}
