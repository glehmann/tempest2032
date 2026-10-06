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
| Left arrow or A | Move clockwise |
| Right arrow or D | Move counterclockwise |
| Enter, numpad Enter, or Space | Fire, hold for repeat fire |
| J or Shift | Jump, after collecting the jump upgrade |
| R | Restart level one |
| Escape | Quit |

Hold a movement key to accelerate around the rim. The 60 Hz simulation uses
the original 16:16 movement values, acceleration `0x400` per tick and maximum
velocity `0x6000`. That reaches 22.5 lanes per second after 24 ticks.
Movement wraps around all 16 lanes and stops when the key is released.
Changing direction stops the claw for one tick before accelerating backwards.
Opposite directions held together cancel each other.

These directions follow the original `claw_right` and `domove`: Right adds to
the original web's counterclockwise lane order. This renderer reverses the
vertex order, so it also reverses the input sign.

Shots launch from the current lane and travel straight down the tunnel.
They stay in their launch lane as the claw moves. The normal weapon fires
every four simulation ticks, with up to eight shots in flight, and removes
each shot at the rear opening. A full magazine skips that firing opportunity
and waits for the next one. Releasing Fire pauses the cooldown.

A shot is not a flat green dot. Sampling a reference screenshot shows three
layers: a near-white core in the shape of the original's `shot` vector, a
bright cyan-green body, and a darker green rim. The original gets this by
drawing a small shape at full brightness and letting bloom spread the colour,
so this prototype builds the layers into the mesh instead. The shades,
`(236, 250, 254)`, `(14, 250, 155)`, and `(0, 154, 53)`, are sampled from that
screenshot.

The shot's own colour register could not explain this. `frab` starts a shot at
hue `$88` and `player_shot` adds `$30` to the high byte of the colour word every
frame, but decoding that cycle through the Jaguar's CRY tables yields blues,
magentas, and oranges, never the green the original displays. The green is
therefore matched to the screenshot, not derived from the registers.

## First-level enemies

Level one uses the original `_nw1` wave of normal Flippers. Their X-shaped
bodies use `s_flipper`'s seven vertices and four triangular faces. The GPU
routine confirms the vertex table is indexed from zero, so the centre, the two
inner points, and the four tips map as listed.

Shading is matched to the original renderer rather than to a literal reading of
the data. `s_flipper` gives each face a CRY hue of `F0` or `F4` plus a
per-vertex intensity, which suggests a Gouraud gradient from dark arm tips to a
bright centre. Sampling a real screenshot shows otherwise: a Flipper renders
as four flat pure-red facets, with no green channel and no gradient along an
arm. So each face takes one flat shade from its brightest source intensity,
scaled so the brightest facet reaches 240 of red. This is a deliberate
departure from the raw data, chosen because it reproduces what the game
actually displays.

The proportions are adjusted for the same reason. A Flipper's four tips sit on
the corners of its bounding box, so that box decides the whole silhouette.
`fverts` uses 16 by 18, which is nearly square, so
the X closes towards a plus sign and each arm points at a corner. Measuring
both references shows a much shallower cross, with the arms meeting at about
58 degrees where ours meet at 97. A box twice as wide as it is tall gives 53
degrees, so this prototype
stretches the tips to `(0, 0)` and `(32, 16)`. That is the third deliberate
departure from `fverts`.

The inner vertices keep the source's proportion rather than its coordinates.
In `fverts` they sit halfway between the centre and a tip, which makes each arm
about a quarter as wide as it is long. The reference measures the same ratio,
so here they move to `(8, 8)` and `(24, 8)`, again halfway out.
Flippers first approach as small red points from beyond the rear opening.
The point is a camera-facing square, kept at two original framebuffer pixels
across. This matches `draw_pel` and `snglpix`, which draw a 2-by-2 block with
depth-cued intensity rather than a shrinking, full-brightness sphere. It starts
dim and brightens as it approaches the web.
Their approach takes 150 ticks, then they climb their lane at the original
`0xA000` depth speed. At the rim they pivot about a lane boundary toward the
player, taking the shortest route around the closed web. A pivot advances
four of the Jaguar's 256 angle units per tick. Their between-flip pause counter
starts at eight and advances to the next flip on underflow. They
can be shot in the destination lane during the latter part of a flip. During
the first part they can grab a player in the departure lane. Once shootable,
they also threaten the destination lane. Shot hits resolve before grabs, so a
shot can save the player, but firing gaps leave the player exposed. Shot
collisions use the original strict six-unit depth range
at the current shot position, so the gaps between shots remain intact.
The weapon tracks active firing across the normal delay between shots, blocking
Flipper grabs until Fire is released or a firing attempt hits a full magazine.
After a magazine gap, the next emitted shot restores that protection. Enemy
shots still need to be intercepted or avoided.

The first spawn is after 61 ticks, with further spawns every 121 ticks when
there is room. The source's duration counter yields 25 enemies. Clearing all
of them completes the level. Each Flipper is worth 150 points.

Climbing Flippers also fire at the player. Level one allows one enemy shot
in flight. It uses `s_shot`'s three rotating chevrons, shaded with the original
purple and yellow-green CRY colors. Enemy shots travel toward the rim at
`0x18000` depth units per tick, and your shots can intercept them.

Destroying a Flipper replaces it with the original's ring explosion. The
source turns the enemy into a `dpring` object, which draws two rings of single
pixels for 51 ticks: an inner ring of 16 pixels and an outer one of 14, at twice
the radius and four units further up the tube. Both rings start green and sweep
along `dat/sines.dat` through `pulser`, so the inner ring only reaches lime
while the outer one carries on into orange. Every frame of that sequence is
prebuilt here. Each explosion adjusts its dot geometry for camera distance and
projection, targeting one pixel at the original 384-pixel framebuffer width.
This keeps distant dots visible instead of shrinking them with the ring.

A Flipper grab shows "CAUGHT YOU" and locks movement, jumping, and firing.
Following `take_me_to_your_leader`, the Flipper and claw keep their grab poses
and travel down the web together at two depth units per tick, reaching the
rear after 80 ticks. The camera stays at the rim during this descent.
`snatch_it_away` then pulls the camera back at eight depth units per tick until
the original viewpoint threshold of -400, about 50 more ticks with this camera.
Only then is the life deducted and the wave reset. R cancels the sequence and
restarts immediately.

Enemy shots cost a life immediately. You start with three lives. Losing a life
resets the wave and gives two seconds of invulnerability.
Press **R** to restart, or replay after clearing the level. This prototype
replays level one rather than advancing to another web.

## Power-ups

Level one's `pup_stuff` entry is `$0404`, but `stdinit` clears the active byte.
`changex` decrements it and drops a power-up on underflow. The first kill drops
one, then kills 6, 11, 16, and 21. A drop replaces that enemy's normal explosion;
intercepted enemy shots and superzap kills do not advance the drop counter.

`draw_pup1` first shows the `opupring` formation animation: a contracting
20-dot ring and four expanding 24-dot rings. The pickup stays at the kill's
lane and depth for 76 ticks while `run_pup` reduces its formation radius from
300 by four each tick. It then travels toward the rim at `0x18000` depth units
per tick. Its body uses `pwrlaser`'s two lightning-bolt triangles, hue `$8a`,
and vertex intensities 128, 192, and 255. Four 12-dot rings trail it at eight-unit
depth intervals, rotating and changing colour through `pulser`. `src/cry.rs`
decodes these colours with all three Jaguar TOM channel tables.

Collection requires the player to be grounded in the same lane, with the
pickup between zero and 32 depth units ahead. The bolt disappears, and the
four rings expand at five radius units per tick for 49 ticks. A missed pickup
passes the player and expires 80 units beyond the rim. Missing one leaves the
reward sequence unchanged.

| Collection | Reward |
| --- | --- |
| 1 | Particle laser: rotating rings of four color-cycling dots, shot speed 4.5 instead of 3.5 units per tick, a three-tick firing interval instead of four, and shots that pass through enemies |
| 2 | Surprise: normally 2,000 points; a 9-in-256 chance of 3,000 points and a level skip |
| 3 | Jump enabled: J or Shift lifts the claw off the rim, avoiding enemies and shots until landing |
| 4 | Another surprise bonus |
| 5 | AI droid: a rotating, colour-cycling wireframe cube that seeks the closest enemy, crosses one lane in 16 ticks, fires every six ticks, and adds eight bullet slots |
| 6 | Superzap plus a warp token |
| 7 onward | Surprise bonuses |

The particle laser uses `draw_pring` and the GPU's `pring`, not the four-wing
`pshot` polygon. Its dots stay two original framebuffer pixels wide, rotate
with camera-relative depth, and use `pulser` for their color sweep.

Jump presses are buffered between simulation ticks and work while firing.
The jump uses `sclawr`'s initial velocity of two units per tick and acceleration
`0xc11`; the camera follows its height. Losing a life removes the laser, jump,
and droid and restarts the collection sequence. Restarting with R clears all
power-ups. The wave waits for remaining pickups to be collected or missed
before displaying level clear.

The original enters a bonus round after accumulating three warp tokens across
waves. Token tracking is included here, but this prototype still replays level
one and has no bonus-round transition. Level one's 25 enemies normally provide
at most five drops.

## Development

The background stars move continuously toward the viewer. `gamefx` advances
the original star-field viewpoint by two units per tick, and `starfield1` in
`src/gpu/llama.gas` wraps relative depth over 512 units. Perspective makes the
stars spread outward faster as they approach, while their brightness rises
with proximity. Here 180 seeded stars use those motion and brightness rules,
with screen-sized square dots drawn behind the web. Camera movement also
shifts their projected positions.

```sh
cargo fmt --check
cargo check
cargo test
cargo clippy -- -D warnings
```

`src/main.rs` contains the scene setup, claw movement, and firing systems.
`src/enemies.rs` contains the Flipper geometry, wave generator, enemy state
machine, collisions, score, and lives. `src/explosion.rs` contains the death
rings, the sine table, and the CRY palette lookup behind them. `src/stars.rs`
contains the moving background star field. `src/powerups.rs`, a child module of
`enemies`, contains drops, collection effects, rewards, jumping, and the droid.
The prototype has no audio.
Actors render in a separate camera pass over the web, matching the original
renderer's draw order and keeping complete enemy silhouettes visible.

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

The six-second sequence includes firing, acceleration in both directions,
stopping, wrapping, and camera settling. It saves screenshots roughly every two
simulation ticks, then exits. Use a fresh output directory for each recording.
The encoded video is a visual preview; screenshot timing can vary with GPU load.
An optional duration extends the capture, for example
`cargo run -- --record-motion target/enemies 15` to see enemies reach the rim.

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
- `sclawr`, `frab`, and `player_shot` control repeat firing and shot travel.
- `_nw1`, `setnewgen`, and `newgen` define level-one enemies and spawn timing.
- `s_flipper` and `fverts` define the solid Flipper faces and vertices.
- `make_flipper`, `ipix`, and `r_obj` define the distant-point approach.
- `fz_stuff`, `rospeeds`, and `pauses` define the level-one movement parameters.
- `stdrail`, `flip_set`, `flipto`, and `stdstop` implement climbing and rim pivots.
- `collie` defines shot vulnerability and the six-unit depth collision threshold.
- `gotu`, `take_me_to_your_leader`, and `snatch_it_away` define the Flipper grab,
  joint descent, camera retreat, and delayed loss of a life.
- `scores` assigns 150 points to a normal Flipper.
- `alienfire`, `run_ashot`, `s_shot`, and `chevron` define enemy firing and shots.
- `killme`, `sphkillme`, and `kime` turn a killed Flipper into its explosion.
- `dpring`, `draw_prex`, and `run_prex` build, draw, and age the ring explosion.
- `pulser` reads `dat/sines.dat` to pick each ring's colour.
- `pring2` in `src/gpu/xcamel.gas` plots a ring a pixel at a time.
- `pup_stuff`, `changex`, and `run_pup` control drops and collection.
- `draw_pup1`, `opupring`, `pupring`, and `pwrlaser` define the pickup visuals.
- `pupvex`, `hilaser`, `suprise`, `jenab`, `addroid`, and `sprzap` define rewards.
- `sclawr`, `mkdroid`, and `rundroid` implement jumping and droid assistance.

`pring2` also computes `255 - radius >> 16` as a ring's intensity, which looks
like a fade as the ring grows. It cannot be one: by that point it has already
divided the radius by z, so the shifted byte is always zero and every ring draws
at full intensity. The colour sweep is the whole of the apparent fade.

The CRY hue-to-RGB lookup used to check those colors comes from
[Virtual Jaguar's TOM implementation](https://github.com/libretro/virtualjaguar-libretro/blob/master/src/tom/tom.c).
It decodes `F4` to a red-orange, but the reference screenshot shows no green on
a Flipper at all, which is why this prototype does not use that value.

The camera framing and colors are tuned to the supplied screenshot rather
than reproducing the Jaguar's renderer exactly.
