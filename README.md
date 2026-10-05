# T2K

A small Tempest 2000-inspired desktop prototype written in Rust and Bevy 0.18.
The first level uses the original game's irregular 16-sided `web5` outline
and straight extrusion. Blue lane surfaces fade into the tunnel, and yellow
edges mark the glider's current lane. The solid gold glider uses the original
`sclaw0` through `sclaw7` vertex layouts, with shaded faces and a thin extruded
body. The claw stays at each lane's midpoint while fractional movement progress
selects its rolling pose. The pose order accounts for this prototype's reversed
vertex ordering. It holds its pose when stopped.

An offset perspective camera follows half the claw's displacement at a capped
speed, so it continues settling after the claw stops. A sparse colored
starfield fills the background. All geometry is generated in code, and no
external assets are needed.

## Run

Install Rust 1.89 or newer, then run from this directory:

```sh
cargo run
```

The first build compiles Bevy and takes longer than subsequent builds.
For an optimized build, use `cargo run --release`.

On Linux, the build needs a C/C++ toolchain and X11 and Wayland development
libraries. On Debian or Ubuntu, install `build-essential`, `pkg-config`,
`libx11-dev`, `libxi-dev`, `libxrandr-dev`, `libxcursor-dev`,
`libxkbcommon-dev`, and `libwayland-dev`. Running needs a graphical desktop
and a Vulkan-capable graphics driver. Windows and macOS use their native
window backends.

## Controls

| Key | Action |
| --- | --- |
| Left arrow or A | Move counterclockwise |
| Right arrow or D | Move clockwise |
| Escape | Quit |

Hold a movement key to accelerate around the rim. The 60 Hz simulation uses
the original 16:16 movement values, acceleration `0x400` per tick and maximum
velocity `0x6000`. That reaches 22.5 lanes per second after 24 ticks.
Movement wraps around all 16 lanes and stops when the key is released.
Changing direction stops the claw for one tick before accelerating backwards.
Opposite directions held together cancel each other.

## Development

```sh
cargo fmt --check
cargo check
cargo test
cargo clippy -- -D warnings
```

`src/main.rs` contains the scene setup and movement systems. Tunnel dimensions
and movement settings are constants at the top of the file. This first slice
has no enemies, shooting, scoring, or audio.

To capture the level for visual comparisons, run:

```sh
cargo run -- --screenshot target/level-01.png
```

The app saves the window after 60 frames and exits. The output directory
must already exist.

To record a scripted movement comparison, run:

```sh
cargo run -- --record-motion target/motion
ffmpeg -framerate 30 -i target/motion/%04d.png -vf "pad=ceil(iw/2)*2:ceil(ih/2)*2" -c:v libx264 -pix_fmt yuv420p target/motion.mp4
```

The six-second sequence includes acceleration in both directions, stopping,
wrapping, and camera settling. It saves screenshots roughly every two
simulation ticks, then exits. Use a fresh output directory for each recording.
The encoded video is a visual preview; screenshot timing can vary with GPU load.

## Original geometry references

The source references used for this prototype are in the original game's
`src/yak.s` and `src/obj2d.s`:

- `raw_webs` selects `web5` for the first level.
- `web5` defines the 16 ring vertices on a 2-to-14 coordinate grid.
- `extrude` repeats the same XY coordinates at a depth of 40 units.
- `vp_set` and `vp_xform` move the viewpoint toward the player's position.
- `clawcon` accelerates the claw and stops it before reversing direction.
- `domove` positions the claw at the selected lane's midpoint.
- `sclaw0` through `sclaw7` define the solid claw's movement poses.
- `dsclaw` selects one of those eight poses from the fractional lane position.

The camera framing and colors are tuned to the supplied screenshot rather
than reproducing the Jaguar's renderer exactly.
