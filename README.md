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
starfield fills the background. Geometry is generated in code from editable
level definitions; no external models or textures are needed.

## Run

Install Rust 1.89 or newer, then run from this directory:

```sh
cargo run
```

The first build compiles Bevy and takes longer than subsequent builds.
For an optimized build, use `cargo run --release`.
Run `cargo run -- --help` to list command-line options, or `--version` to show
the version. Screenshot and recording modes are mutually exclusive; recordings
accept an optional positive duration in whole seconds after the directory.
Pass `--fullscreen` to start borderless fullscreen on the primary monitor
instead of a window, and `--fps` to show the frame rate in the top-right corner
under the level label. The label refreshes every 0.5 seconds and shows the
fastest and slowest frame times in milliseconds over the last two seconds.
Every frame counts, so a single hitch appears at the next refresh and stays
in the range until it ages out. The same line reports this process's average
CPU usage as a percentage of total CPU capacity and its resident memory in
MiB, once their first readings arrive.

Presentation uses FIFO VSync to avoid tearing. With `--fps`, late application
updates also produce warnings when their interval exceeds 1.5 display refresh
periods, or 25 ms at 60 Hz. The threshold follows the window's current monitor;
if its refresh rate is unavailable, the log states that 60 Hz is assumed.
Warnings are limited to once per second and include the number of late
intervals and the worst interval. Unfocused, hidden, and minimized windows
are ignored, as is the first interval after resuming or changing refresh rate.
These warnings indicate possible missed refreshes. Bevy does not expose actual
display presentation timing, so they cannot confirm or detect every dropped
frame. Simulation runs at 60 Hz without render interpolation.

On Linux, the build needs a C/C++ toolchain and X11 and Wayland development
libraries. On Debian or Ubuntu, install `build-essential`, `pkg-config`,
`libx11-dev`, `libxi-dev`, `libxrandr-dev`, `libxcursor-dev`,
`libxkbcommon-dev`, and `libwayland-dev`. Running needs a graphical desktop
and a Vulkan-capable graphics driver. Windows and macOS use their native
window backends.

## Editable levels

Levels load from `levels/*.toml` at startup. Edit the files and relaunch the app;
no Rust changes are needed. The supplied levels are:

- `levels/01.toml`: the existing `web5` level, with 25 Flippers, first spawn after
  61 gameplay ticks, and subsequent spawns every 121 ticks.
- `levels/02.toml`: the original `_nw2` on the square `web11`, with 16 lanes,
  25 Flippers, first spawn after 51 ticks, and subsequent spawns every 101 ticks.
  The Flippers climb at the same `0xA000` speed as level one. They fold through
  180 degrees on straight sides and 90 degrees around corners, using the
  original per-lane orientations.
- `levels/03.toml`: the original `_nw3` on the open, eleven-lane flat `web1`.
  Flippers and Flipper Tankers share a 17-enemy generation budget. Tankers
  arrive first after 223 gameplay ticks, then every 445 ticks. Their pink
  `s_fliptank` shells use the original vertices and faces, with a Flipper motif
  inside. Shooting one or letting it reach the rim awards 100 points and sends
  two Flippers into neighboring lanes. They finish their initial flip before
  climbing; the original 8-in-256 mutation chance produces fast blue Flippers
  that scatter further before returning to normal pursuit.
- `levels/04.toml`: the open fifteen-lane V, `web2`, with a shared 25-spawn
  budget for Flippers, Tankers, and Spikers. A Spiker uses the original rotating
  spiral outline and creates a 30-unit spike in a free lane, or extends the
  shortest existing spike when every lane is occupied. It makes 51 extensions
  at `0xD000` speed, retreats, and picks another spike. Only one Spiker is
  active at a time. Shooting a spike removes three units and scores three
  points; particle shots continue through it. Destroying the final segment
  scores another three points. Spikes remain after their builder dies and do
  not block level completion, but can kill the glider during the exit flight.
  The exit displays "AVOID THE SPIKES" and still permits movement and firing
  until the rear. A spike collision pulls the camera back before losing a life.
- `levels/05.toml`: the eighteen-lane triangle `web9`, with a 25-spawn budget,
  `0xA400` Flippers, `0x8800` Tankers, and `0xE000` Spikers. Tankers repeat
  every 281 ticks. Pickups drop on the first kill and then every sixth kill,
  matching the next `pup_stuff` group.
- `levels/06.toml`: the open fifteen-lane U-shaped `web3`, with a 31-spawn
  budget. Flippers repeat every 96 ticks and Tankers every 261 ticks, retaining
  level five's climbing speeds. Spikers repeat every 801 ticks.
- `levels/07.toml`: Tempest 2000's fourteen-lane lemon-shaped `web18`, with a
  31-spawn budget, `0xA600` Flippers, and `0xF000` Spikers. The T2K mode adds
  16 to the web-table index, so this is `web18`, not classic mode's `web12`.
  Flippers repeat every 91 ticks, Tankers every 251, and Spikers every 751.
- `levels/08.toml`: the same T2K lemon and 31-spawn budget, with Tankers every
  241 ticks and Spikers every 651 ticks. Its Tanker-child mutation threshold
  rises to 28 out of 256.
- `levels/09.toml`: the open fourteen-lane sine-wave `web13`, with a 31-spawn
  budget, `0xA800` Flippers, and `0x10000` Spikers. Two alien shots can now be
  active at once, following the original `ashots` table calculation. Tankers
  repeat every 231 ticks and pickups every seventh kill after the first.
- `levels/10.toml`: the closed clover `web4`, introducing Beasties every 2,001
  ticks. Their original `hornm1` core and two `hornm2`/`hornm3` armor layers are
  drawn together, with the core scaled to meet the horn roots. Rail hits strip
  horns before killing the core; stripped
  horns fly back at half shot speed, occupy their original player shot slots,
  and absorb ordinary shots. Beasties pursue without the normal rim pause.
- `levels/11.toml`: the open parabolic `web14`, introducing Fuseballs. Their
  five independently shaped colored legs use `fbpiece1`/`fbpiece2` and the
  original five hue values. They climb along lane boundaries for 49 ticks,
  then cross in sixteen steps with a five-tick delay per step. Only the middle
  of a crossing is shootable. A crossing at the rim can fry the glider even
  during active firing, using the original "FRIED YOU!" death sequence.
  Kills award the original random 250, 500, or 750 points without a pickup drop.
- `levels/12.toml`: the closed cross `web10`, including the original inward
  corner orientations. Flippers repeat every 81 ticks and Fuseballs every 701.
- `levels/13.toml`: T2K's open giraffe-neck `web20`, with thirteen lanes and
  `0xAC00` Flippers. This mode-specific selection differs from classic `web15`.
  Fuseballs climb at `0x8800` and repeat every 601 ticks.
- `levels/14.toml`: the open distorted-W `web6`, with fifteen lanes and
  Flippers every 76 ticks. The local flat sections retain their source angles.
- `levels/15.toml`: open `web16`, with `0xAE00` Flippers and `0x13000` Spikers.
  Fuseballs climb at `0x9000` and repeat every 501 ticks.
- `levels/16.toml`: T2K's compact eight-lane `web21`, with a 41-spawn budget,
  Flippers every 71 ticks, and the original 60-in-256 fast-child threshold.
- `levels/17.toml`: the eighteen-lane big oval `web24`, starting T2K's second
  web set with red surfaces and cyan Flippers. The 25-spawn wave includes
  Pulsars every 601 ticks, Spikers every 751, Beasties every 1,001, and
  Fuseballs every 901. There are no Flipper Tankers, and three enemy shots
  can be active at once. Flippers return to the `0xA800` speed group.
  Pulsars climb at `0x8000` and share a sixteen-phase cycle, advancing every
  nine ticks. Phase seven electrifies the occupied lane, frying a grounded
  glider even while it fires. Jumping and respawn invulnerability protect it.
  Pulsars wait for a safe phase before entering the web, then split into two
  rim sparks that move in opposite directions every five ticks, wrapping on
  closed webs and bouncing at open ends. Rail kills award 200 points and
  spark kills 150; both participate in the normal pickup-drop sequence.
- `levels/18.toml`: the open fifteen-lane spiraloid `web23`, with a 31-spawn
  budget. Flippers repeat every 101 ticks and Pulsars every 501. It retains
  level seventeen's red surfaces, cyan Flippers, climbing speeds, and other
  generators. Pulsar sparks bounce at the open ends.
- `levels/19.toml`: the closed fifteen-lane pentagon `web27`, with a 31-spawn
  budget. Flipper Tankers return every 445 ticks with a 72-in-256 fast-child
  threshold. Flippers climb at `0xAA00`, Fuseballs at `0x9800`, and Pulsars
  repeat every 601 ticks.
- `levels/20.toml`: the open twelve-lane hook `web25`, with a 33-spawn budget.
  Flippers repeat every 81 ticks, Tankers every 334, and Pulsars every 501.
  The Tanker-child mutation threshold rises to 76 out of 256. Climbing speeds
  stay in level nineteen's group, and Pulsar sparks bounce at the open ends.
- `levels/21.toml`: compact eight-lane `web21`, with 33 shared spawns,
  `0xAC00` Flippers, and Pulsars every 433 ticks. Their phase delay drops
  to eight ticks. Tankers repeat every 281 ticks with an 80-in-256 mutation
  threshold; Spikers climb at `0xE000` and Fuseballs at `0xA000`.
- `levels/22.toml`: open twelve-lane inverted-bird `web28`, with 36 shared
  spawns. Flippers repeat every 96 ticks, Tankers every 261, and Pulsars every
  426. Pulsar sparks bounce at the wing tips.
- `levels/23.toml`: closed sixteen-lane tiny-star `web26`, retaining its
  retraced tips and sharp lane reversals. Flippers climb at `0xAE00`, Spikers
  at `0xF000`, and Fuseballs at `0xA800`; Pulsars repeat every 463 ticks.
- `levels/24.toml`: closed fifteen-lane D-shaped `web29`, with six flat lanes
  along its base. Tankers repeat every 241 ticks, Spikers every 651, and
  Pulsars every 412, with the same 36-spawn budget as levels 22 and 23.
- `levels/25.toml`: open thirteen-lane curved-L `web30`, with 41 shared spawns
  and Fuseball Tankers every 2,001 ticks. Their rotating five-armed graphic
  draws both original leg shapes per arm. Shooting a Tanker or letting it
  reach the rim awards 100 points and releases two Fuseballs at its depth,
  crossing outward from the lane's two edges. Children do not consume the
  generation budget, and opening the Tanker does not advance the pickup counter.
  Pulsar phases now last seven ticks and pickups drop every eighth kill.
- `levels/26.toml`: open seventeen-lane mouse-pointer `web31`, including its
  folded tail. Pulsars repeat every 400 ticks and Spikers every 851; Fuseball
  Tankers retain their 2,001-tick interval.
- `levels/27.toml`: open twelve-lane diagonal double-curve `web32`, with
  `0xB200` Flippers, `0x11000` Spikers, and `0xB800` Fuseballs. Pulsars repeat
  every 389 ticks and Tankers every 371.
- `levels/28.toml`: closed sixteen-lane cat-head `web33`, with inward-facing
  ear corners. Flippers repeat every 81 ticks, Pulsars every 378, and
  Fuseballs every 701. The fast-child threshold rises to 108 out of 256.
- `levels/29.toml`: closed fifteen-lane teardrop `web34`, retaining the
  retraced top tip. Flippers climb at `0xB400`, Tankers at `0x9800`, Spikers
  at `0x12000`, and Fuseballs at `0xC000`. Pulsar phases drop to six ticks.
- `levels/30.toml`: open fourteen-lane crinkly-L `web35`, with alternating
  angles along its stem. Flippers repeat every 76 ticks, Pulsars every 351,
  and Fuseball Tankers every 1,001, twice their previous frequency.
- `levels/31.toml`: closed thirteen-lane kiss-of-death `web36`, with the
  original shallow inward corners. Flippers climb at `0xB600`, Spikers at
  `0x13000`, and Fuseballs at `0xC800`; Flippers repeat every 96 ticks.
- `levels/32.toml`: open fifteen-lane backwards-C `web37`, finishing the
  second web set. Fuseballs and Fuseball Tankers both repeat every 501 ticks,
  while Pulsars repeat every 901. Four enemy shots can be active at once.
- `levels/33.toml`: returns to sixteen-lane `web5`, now with yellow surfaces
  and yellow-green Flippers. The 25-spawn wave introduces Pulsar Tankers
  every 501 ticks and Beasties every 701. A Pulsar Tanker draws two opposing,
  rotating zigzags synchronized with the shared pulse. Opening it awards 100
  points and releases two children that flip into adjacent lanes before
  climbing at `0x8000` and electrifying them. There are no direct Pulsar,
  Flipper Tanker, or Fuseball Tanker generators in this wave. Pickups now
  drop every ninth kill after the first.
- `levels/34.toml`: square `web11`, with 31 shared spawns and Pulsar Tankers
  every 451 ticks. Beasties retain their 701-tick interval.
- `levels/35.toml`: open eleven-lane flat `web1`, restoring Flipper Tankers
  every 445 ticks with a 136-in-256 mutation threshold. Flippers climb at
  `0xB200` and Fuseballs at `0xA800`.
- `levels/36.toml`: open fifteen-lane V-shaped `web2`, with 33 shared spawns.
  Flippers repeat every 81 ticks, Flipper Tankers every 334, Pulsar Tankers
  every 401, and Fuseballs every 951.
- `levels/37.toml`: eighteen-lane triangle `web9`, with `0xB400` Flippers,
  `0x9800` Tankers, and `0xB000` Fuseballs. Pulsar phases drop to six ticks;
  Spikers repeat every 801 ticks.
- `levels/38.toml`: open fifteen-lane U-shaped `web3`, with 36 shared spawns.
  Flippers repeat every 96 ticks and Flipper Tankers every 261, while Pulsar
  Tankers retain their 451-tick interval.
- `levels/39.toml`: fourteen-lane T2K lemon `web18`, with `0xB600` Flippers,
  `0xF000` Spikers, and `0xB800` Fuseballs. Pulsar Tankers repeat every 401
  ticks and Flipper Tankers every 251.
- `levels/40.toml`: the same T2K lemon, with Pulsar Tankers every 351 ticks,
  Flipper Tankers every 241, and Spikers every 651. The Flipper Tanker
  mutation threshold rises to 156 out of 256.
- `levels/41.toml`: open fourteen-lane sine-wave `web13`, with 41 shared
  spawns and all three Tanker types. Pulsar Tankers repeat every 351 ticks,
  Fuseball Tankers every 801, and direct Pulsars every 1,101. The shared
  Pulsar phase delay drops to five ticks.
- `levels/42.toml`: sixteen-lane clover `web4`, retaining its inward corners.
  Beasties repeat every 701 ticks, Spikers every 851, and Fuseball Tankers
  every 2,001. Direct Pulsars still repeat every 1,101 ticks.
- `levels/43.toml`: open thirteen-lane parabolic `web14`, with `0xBA00`
  Flippers and `0xC800` Fuseballs. Fuseball Tankers repeat every 301 ticks,
  Fuseballs every 401, and Pulsar Tankers every 501.
- `levels/44.toml`: sixteen-lane cross `web10`. Direct Pulsars and Pulsar
  Tankers both repeat every 501 ticks; Fuseball Tankers remain on their
  301-tick interval, and Flippers repeat every 81 ticks.
- `levels/45.toml`: open thirteen-lane T2K giraffe-neck `web20`, with
  `0xBC00` Flippers and `0xB800` Tankers. Pulsar phases drop to four ticks;
  Pulsar Tankers repeat every 301 ticks and direct Pulsars every 1,001.
- `levels/46.toml`: open fifteen-lane distorted-W `web6`, with Flippers every
  76 ticks and a 180-in-256 fast-child threshold. Other generator intervals
  and speeds remain in level forty-five's group.
- `levels/47.toml`: open fifteen-lane `web16`, with `0xBE00` Flippers,
  `0x13000` Spikers, and `0xD800` Fuseballs. Direct Pulsars now repeat every
  351 ticks while Pulsar Tankers remain at 301.
- `levels/48.toml`: compact eight-lane `web21`, finishing the third web set.
  Flipper Tankers repeat every 231 ticks, Pulsar Tankers every 301, and
  Fuseball Tankers every 401; the fast-child threshold reaches 188 out of 256.
- `levels/49.toml`: eighteen-lane big oval `web24`, starting the fourth web
  set with blue surfaces and pale-cyan Flipper colors. This wave consists
  entirely of 17 armored Beasties: first spawn after 71 gameplay ticks,
  then every 141 ticks, climbing at `0xB000`. Normal Flipper generation is
  disabled, and there are no other enemy generators.
- `levels/50.toml`: open fifteen-lane spiraloid `web23`, restoring normal
  Flippers every 51 ticks in a 41-spawn mixed wave. Beasties repeat every
  501 ticks, Spikers every 751, and Fuseballs every 901.
- `levels/51.toml`: closed fifteen-lane pentagon `web27`, with 46 shared
  spawns. Pulsar Tankers repeat every 951 ticks in place of Fuseballs;
  Flippers climb at `0xB200`, and Beasties repeat every 501 ticks.
- `levels/52.toml`: open twelve-lane hook `web25`, increasing the shared
  budget to 51 spawns. Beasties repeat every 301 ticks, Fuseballs every 951,
  and normal Flippers continue every 51 ticks. Pulsar Tankers are absent.
- `levels/53.toml`: compact eight-lane `web21`, restoring Flipper Tankers
  every 281 ticks with a 208-in-256 fast-child threshold. Beasties repeat
  every 201 ticks; Flippers climb at `0xB400` and Spikers at `0xE000`.
- `levels/54.toml`: open twelve-lane inverted-bird `web28`, with Beasties
  every 101 ticks and Flipper Tankers every 261. The shared budget stays
  at 51 spawns, with a 212-in-256 fast-child threshold.
- `levels/55.toml`: closed sixteen-lane tiny-star `web26`, preserving its
  retraced tips. Flippers and Beasties climb at `0xB600`, Spikers at `0xF000`,
  and Fuseballs at `0xB800`; Flipper Tankers repeat every 251 ticks.
- `levels/56.toml`: closed fifteen-lane D-shaped `web29`, with Flippers every
  51 ticks, Beasties every 101, Flipper Tankers every 241, and Spikers every
  651. The fast-child threshold reaches 220 out of 256.
- `levels/57.toml`: open curved-L `web30`, with 61 shared spawns. Dedicated
  Super Flippers appear first after 13 ticks, then every 26, using the blue
  `supf1` mesh and double-speed rim flips without a normal pause. Mirrors
  repeat every 401 ticks, spin, and stop forty units from the rim. Surviving
  hits push a Mirror backward by sixteen climbing steps and return the shot
  at full speed, retaining its magazine slot. Five hits or recoil beyond
  the rear destroys it for 250, 500, or 750 points without a pickup drop.
- `levels/58.toml`: open seventeen-lane mouse-pointer `web31`, with 41 shared
  generation attempts. The executable wave table selects Spikers every 51
  ticks, despite its "Flippers" comment. Only one Spiker can be active;
  blocked attempts still count against the budget, as in `launch1`.
  Mirrors repeat every 401 ticks and Pulsars every 152, with five-tick phases.
- `levels/59.toml`: open twelve-lane diagonal double-curve `web32`, adding
  Fuseballs every 524 ticks. Mirrors and Fuseballs use the `0xC800` climbing
  speed group, while the primary Spiker attempts retain their 51-tick interval.
- `levels/60.toml`: closed cat-head `web33`, with 51 shared attempts and
  a second Spiker generator every 1,501 ticks. Both Spiker timers share the
  original one-active-Spiker limit; blocked attempts still consume budget.
  Mirrors, Pulsars, and Fuseballs keep level fifty-nine's intervals.
- `levels/61.toml`: closed teardrop `web34`, retaining the retraced tip.
  Mirrors and Fuseballs climb at `0xD000`, Spikers at `0x12000`, and Pulsar
  phases drop to four ticks. Generator intervals remain those of level 60.
- `levels/62.toml`: open fourteen-lane crinkly-L `web35`, replacing direct
  Pulsars with Pulsar Tankers every 352 ticks. Both Spiker timers and the
  Mirror and Fuseball generators retain their previous intervals.
- `levels/63.toml`: closed thirteen-lane kiss-of-death `web36`, with Pulsar
  Tankers every 252 ticks. Mirrors and Fuseballs climb at `0xD800`, while
  Spikers use the final fourth-set `0x13000` speed group.
- `levels/64.toml`: open fifteen-lane backwards-C `web37`, ending the fourth
  web set with primary Pulsars every 31 ticks and Beasties every 401. Normal
  Flippers, Mirrors, Spikers, and Tankers are absent from this 51-spawn wave.
- `levels/65.toml`: returns to sixteen-lane `web5`, starting the fifth set
  with magenta surfaces and pale-cream Flippers. The dispatch table selects
  Super Flippers every 21 ticks and Alien Droids every 701. Alien Droids
  climb at twice the Flipper speed to fifteen units beyond the rim, cross
  a lane in sixteen three-tick steps, then zap for 32 ticks. Jumping and
  invulnerability protect the player. Shots repel an ascending Droid by
  sixteen climbing steps; recoil beyond the rear or a hit in its rim phase
  destroys it for bonus points without a pickup drop.
- `levels/66.toml`: square `web11`, with Super Flippers every 41 ticks,
  Mirrors every 751, and Alien Droids every 601. The source's primary
  dispatch remains type 7, despite its "Flippers3" comment.
- `levels/67.toml`: open eleven-lane flat `web1`, adding Fuseballs every 501
  ticks. Alien Droids climb at `0x16400`, twice the `0xB200` Super Flipper
  speed; Mirrors and Fuseballs use `0xA800`.
- `levels/68.toml`: open fifteen-lane V-shaped `web2`, preserving level 67's
  Super Flipper, Mirror, Alien Droid, and Fuseball intervals in a 61-spawn wave.
- `levels/69.toml`: eighteen-lane triangle `web9`, replacing Fuseballs with
  Pulsar Tankers every 501 ticks. Super Flippers climb at `0xB400`, Alien
  Droids at `0x16800`, and the shared Pulsar phase delay is six ticks.
- `levels/70.toml`: open fifteen-lane U-shaped `web3`, with primary Pulsar
  Tankers every 71 ticks instead of Super Flippers. Fuseballs return every
  301 ticks, while Mirrors and Alien Droids retain their previous intervals.
- `levels/71.toml`: fourteen-lane T2K lemon `web18`, replacing Fuseballs with
  Beasties every 501 ticks. Primary Pulsar Tankers remain on their 71-tick
  interval, alongside Mirrors and faster `0x16C00` Alien Droids.
- `levels/72.toml`: the same T2K lemon, restoring Super Flippers every 21
  ticks and adding Spikers every 501. The 61-spawn wave retains Mirrors and
  Alien Droids, while Pulsar Tankers and Beasties are absent.
- `levels/73.toml`: open fourteen-lane sine-wave `web13`, adding Fuseballs
  every 301 ticks to the Super Flipper, Mirror, Alien Droid, and Spiker wave.
  Alien Droids climb at `0x17000` and Spikers at `0x16000`.
- `levels/74.toml`: closed clover `web4`, adding type-3 Super Flippers every
  401 ticks. They use the six-face `s_flip2` mesh, flip away from the player
  while descending, stop descending when aligned, and pursue normally at
  the rim. Unlike type 2, they use the normal four-unit angular step. The
  source's extra firing call is commented out and remains inactive here.
- `levels/75.toml`: open parabolic `web14`, restoring Flipper Tankers every
  701 ticks. The fast-child threshold is capped at 252 out of 256; an
  independent 148-in-256 check can turn a mutated child into type 3.
  Spiral children descend during their release flip and then flee the player.
  The currently configured cycle wraps back to level one after this level.

Clearing a level advances after the exit animation. The last level loops back to
level one, carrying score, lives, and upgrades. A death retries the current
level and removes upgrades. R starts a fresh run from level one.

Start directly on the second level with:

```sh
cargo run -- --level 2
```

Use `--levels path/to/levels` to load another directory. These options can be
combined with screenshot and recording options, for example:
`cargo run -- --level 2 --record-exit target/square-exit`.

The TOML fields use these conventions:

- `number`: levels are loaded in numerical order, numbered consecutively from 1.
- `web.closed`: defaults to true. Open webs have one more vertex than lanes;
  movement stops at their endpoints and enemies cannot wrap across them.
- `web.vertices`: vertices in the prototype's screen order, on the original
  coordinate grid. Closed outlines are clockwise; do not repeat their first
  vertex. Open outlines list both endpoints and can be a straight line.
- `web.orientations`: one Jaguar angle per lane. A full turn is 256 units;
  negative values turn clockwise. These also determine Flipper flip targets.
- `web.start_lane`: a zero-based index in this clockwise order. Original
  `web11` lane 5 is lane 10 here. Level one's existing prototype spawn is kept.
- `web.depth`: depth in game units, 160 for the supplied webs.
- `flippers`: count, first and repeat spawn times in 60 Hz ticks, climbing speed
  in depth units per tick, flip speed in angle units per tick, pause counter,
  and maximum simultaneous Flippers. Climbing speed converts to 16:16 once
  when the file is loaded.
  `flippers.enabled` defaults to true; set it to false to disable normal
  Flipper generation. The table still supplies shared capacity and rim-motion
  settings, including those used by Beasties and Tanker children. A disabled
  Flipper generator requires another enemy generator in the level.
  Optional `super_flippers` and `mirrors` tables use the standard generator
  fields. Super Flippers have independent spawn timers and double-speed flips.
  Optional `spiral_flippers` tables configure the independent type-3 generator.
  Optional `aliens` tables specify Alien Droid generation and climbing speed.
- `enemy_count`: shared generation budget for mixed waves. It replaces the
  legacy `flippers.count` value when supplied. Released Tanker contents do not
  consume this budget. Optional `tankers` tables specify spawn times, climbing
  speed, and the fast-child probability out of 256.
  `tankers.spiral_chance` defaults to zero and sets the second mutation check
  for a child that already passed `super_chance`.
  Optional `spikers` tables specify their spawn times and climbing speed.
  Optional `extra_spikers` tables add a second timer sharing the one-Spiker
  limit and require a `spikers` table with the same climbing speed.
  Optional `fuse_tankers` tables use the same generator fields and require
  a `fuseballs` table to supply their children's climbing speed.
  Optional `pulsars` tables also specify `phase_ticks` for the shared pulse
  and `propagation_ticks` for rim-spark movement. Both must be positive.
  Optional `pulsar_tankers` tables use the same fields, with `climb_speed`
  controlling the Tanker. Released children use the `pulsars` climbing speed
  when present, or the original `0x8000` otherwise. Both tables must agree
  on phase and propagation timing.
- `appearance.surface` and `appearance.lines`: RGB channels between 0 and 1.
  `appearance.flipper` uses integer RGB channels between 0 and 255.
- `powerups.kill_interval`: first kill drops a pickup, then every Nth kill.
  Each new level starts a fresh drop counter while retaining collected rewards.

Invalid geometry, out-of-range values, numbering errors, and unknown fields
are rejected with the offending file's name. Shared player, weapon, palette,
and animation behavior stays in Rust.

## Level entry

The level begins with the original `setweb` and `rotate_web` sequence: a
wireframe web approaches from 256 depth units away at one unit per tick. Over
256 ticks its XZ rotation makes a full turn, while its YZ rotation advances
from half a turn to a full turn. It rotates around the tunnel's centre, leaving
the camera and stars independent of the web.

Once the web settles, its blue faces fade in by eight intensity steps per tick
to 248. The claw rezzes over 64 ticks using `srezclaw`, `dsclaw2`, and `draw_z`:
the original `claw1` through `claw8` outlines draw at four coloured depths,
with the spacing contracting from 16 depth units to zero. Movement and firing
are available during this effect; the enemy wave starts counting down as soon
as the web settles. The solid claw replaces the outlines when rezzing ends.

Restarting with R and re-entering after a lost life replay this sequence.

## Level exit

Clearing the wave starts the original `zoom1` and `czoom1` fly-through. The
camera and claw accelerate down the tunnel using `zs_stuff`'s `0x400` fixed-point
step per tick. The blue faces fade away; movement and firing remain available
until the claw reaches the rear.

At the rear, `czoom2` replaces the solid claw with four spinning wireframe
copies. Their depth spacing grows as they fly away. `zoom2` hides the web and
adds further acceleration to the claw. The starfield uses `starfield1`'s four
depth-spaced samples per star, with separation growing by a quarter unit per
tick. Reaching the source's 2,000-unit claw depth triggers the warp flash.

After the flash, the next configured level begins with score, lives, and
upgrades preserved. The last level loops back to the first. R cancels the exit
and starts a fresh game.

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
Movement wraps around closed webs and stops at the endpoints of open webs.
Releasing the movement key stops the claw.
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

An enemy-shot hit shows "SHOT YOU!" and replaces the glider with `zap`'s
ten-point starburst outline. `draw_z` draws four copies with hues separated by
17 palette entries. Following `zap_player`, the burst spins by 12 angle units
per tick, its copies spread apart by half a depth unit per tick, and its centre
moves toward the viewer by a quarter unit per tick. The camera moves toward
the burst's XY position and pulls back, including `vp_xform`'s depth correction.
The blue web faces fade out during the explosion.

After 121 ticks the starburst disappears, and the camera retreats at eight
depth units per tick until the same -400 viewpoint threshold as a grab. Only
then is the life deducted, followed by re-entry or game over. Movement, jumping,
and firing are locked during the sequence; R cancels it and restarts.

You start with three lives. Losing a life resets the wave and gives two seconds
of invulnerability.
Press **R** to restart on level one. After a level-clear exit, the next web loads
automatically.

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
waves. Token tracking is included here, but this prototype cycles through its
configured levels without a bonus-round transition. Level one normally provides
at most five drops; mixed waves can provide more pickup-eligible kills through
released Tanker contents.

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

Each Rust file starts with module documentation explaining its role and game
terminology. The source is organized by responsibility:

| Module | Contents |
| --- | --- |
| `src/main.rs`, `src/cli.rs` | Entry point and command-line options |
| `src/app.rs` | App construction and ordered input, fixed-tick simulation, and rendering schedules |
| `src/game.rs`, `src/hud.rs` | Game state, score, lives, level loading, retries, restarts, and status messages |
| `src/glider.rs`, `src/weapons.rs` | Player claw movement and poses, firing, bullets, and particle-laser graphics |
| `src/collisions.rs` | Shot hits, armor reflections, shell releases, spike trimming, and kill rewards |
| `src/scene.rs`, `src/camera.rs`, `src/mesh.rs` | Scene setup, web rendering, lane highlights, cameras, and shared mesh helpers |
| `src/web.rs`, `src/level.rs` | Lane topology and geometry, TOML definitions, validation, and the level catalog |
| `src/capture.rs` | Screenshots, recordings, scripted input, and transition previews |
| `src/enemies/` | Flippers, enemy shots, Tankers, Spikers and spikes, Beasties, Fuseballs, and Pulsars with rim sparks |
| `src/powerups/` | Reward state, pickups, particle rings, jumping, and the AI droid |
| `src/animations/` | Web entry, level exit and warp, Flipper capture, and glider death |
| `src/explosion.rs`, `src/cry.rs`, `src/stars.rs` | Enemy-death rings, Jaguar color decoding, and background stars |
| `src/test_support.rs`, `src/**/tests/` | Headless fixtures and focused unit tests, compiled only for testing |

The game uses one loader for advancement, death retries, and restarts,
rebuilding the web and clearing level-owned actors each time. System ordering
is kept together in `app.rs` so movement, firing, collisions, and transitions
retain their fixed-tick timing.
The prototype has no audio.
Actors render in a separate camera pass over the web, matching the original
renderer's draw order and keeping complete enemy silhouettes visible.

To capture the level for visual comparisons, run:

```sh
cargo run -- --screenshot target/level-01.png
```

The app saves the window 60 visible-claw frames after the entry animation and
exits. The output directory must already exist.

To record a scripted movement comparison, run:

```sh
cargo run -- --record-motion target/motion
ffmpeg -framerate 30 -i target/motion/%04d.png -vf "pad=ceil(iw/2)*2:ceil(ih/2)*2" -c:v libx264 -pix_fmt yuv420p target/motion.mp4
```

The recording starts with level entry, then scripted movement and firing.
Its default duration is six seconds, including the 4.27-second web approach.
It saves screenshots roughly every two
simulation ticks, then exits. Use a fresh output directory for each recording.
The encoded video is a visual preview; screenshot timing can vary with GPU load.
An optional duration extends the capture, for example
`cargo run -- --record-motion target/enemies 15` to see enemies reach the rim.

To preview the exit without playing through the wave, run:

```sh
cargo run -- --record-exit target/exit
```

This records 14 seconds and clears the wave six seconds into the capture,
showing entry, the complete exit, the warp flash, and the start of re-entry.
An optional duration works as with `--record-motion`.

To preview the enemy-shot death sequence, run:

```sh
cargo run -- --record-shot-hit target/shot-hit
```

This records 15 seconds and injects an enemy shot six seconds into the capture,
showing entry, the starburst, camera retreat, and re-entry.

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
- `setweb`, `rotate_web`, `srezclaw`, `dsclaw2`, and `draw_z` define level entry
  and the contracting wireframe claw trails.
- `szoom`, `zoom1`, `czoom1`, `zoom2`, and `czoom2` define the level-clear flight
  and expanding claw trails; `starfield1` draws the warp star samples.
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
- `run_ashot`, `ouch`, `zap_player`, `zap`, and `draw_z` define the shot-hit
  message, spinning starburst, depth copies, and retreat before losing a life.
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
