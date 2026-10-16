# Custom level demo

Run this from the repository root:

```sh
cargo run --release -- --levels examples/custom-level --level 1
```

For a downloaded executable, pass the same options with the path to this directory.
The demo replaces level 1 with a slower eight-lane web and more frequent power-ups.
All other levels fall back to the embedded catalog. `--level 1` starts the demo
directly; omit it to enter through Welcome. Custom definitions disable high-score
and player Key saving.

Copy this directory anywhere and edit `01.toml`. The game reads `.toml` files
directly inside the directory given by `--levels`, which defaults to `levels`.
Restart after editing. Filenames are arbitrary; `number` chooses which level to
replace. Each file is a complete definition, not a patch. Duplicate numbers,
invalid files, and gaps after the embedded catalog are errors. You can append
level 101, then 102, and so on.

## Soundtrack

The demo downloads Acid Butcher from Mod Archive on its first launch:

```toml
soundtrack = "https://api.modarchive.org/downloads.php?moduleid=139997#acid_butcher.xm"
```

You can also use a local file. Change this top-level option:

```toml
number = 1
soundtrack = "music/my-track.mp3"
```

Relative paths start at the TOML file's directory. Absolute paths also work.
Native builds support MP3, FLAC, Ogg Vorbis, WAV, MOD, and XM. Formats are detected
from the contents, so download URLs do not need a filename extension. No copy of
the demo music is bundled with this directory.

HTTP and HTTPS URLs download once, then reuse a persistent cache, including
offline. Query parameters are part of the cache key; the `#filename` fragment
is ignored. Downloads are checked before caching and saved atomically. A broken
cache entry is downloaded again. Delete an entry to fetch a new copy at the same
URL. The cache is in your OS cache directory under `tempest2032/soundtracks`:

- Linux: `$XDG_CACHE_HOME/tempest2032/soundtracks`, usually
  `~/.cache/tempest2032/soundtracks`.
- macOS: `~/Library/Caches/tempest2032/soundtracks`.
- Windows: `%LOCALAPPDATA%\tempest2032\soundtracks`.

Omitting `soundtrack`, using a missing or invalid file, or failing to download an
uncached URL uses the original embedded music. Loading failures produce a log
warning. If cache storage is unavailable, downloaded music can still play for
that session. Downloads happen at startup with a 30-second timeout and a 128 MiB
size limit.

Custom tracks loop, use the normal volume controls, and pause with the game.
Consecutive levels referencing the same resolved path or URL keep the track playing.
Changing tracks uses the usual one-second fade-out. Menus, game over, and warp
stages keep their original music. Files are read and checked at startup, so
restart after replacing a local soundtrack. Browser builds use embedded levels and
music and do not load these custom files or URLs.

## TOML options

Place top-level options before the first `[table]`. TOML associates later keys
with the most recent table. Unknown keys are rejected, including misspellings.
Timing values use the 60 Hz simulation. Spawn and pause counters follow the
original game's update rules, so a counter can take an extra tick to expire.
Difficulty settings scale climbing speeds and spawn intervals after loading.

### Top-level fields

| Field | Meaning |
| --- | --- |
| `number` | Required positive level number to replace or append. |
| `soundtrack` | Optional nonempty file path or HTTP/HTTPS URL. See above. |
| `enemy_count` | Shared wave spawn budget, 1–65535. Takes precedence over `flippers.count`. Other generators share this budget. |
| `enemy_shots` | Alien Droid shot limit, 1–16. Defaults to 1 on levels 1–8, 2 on 9–16, 3 on 17–24, and 4 thereafter. |

### `[web]`, required

| Field | Meaning |
| --- | --- |
| `closed` | Whether the last vertex connects to the first. Defaults to `true`. An open web has one fewer lane than vertices. |
| `vertices` | Required `[x, y]` coordinates in the original web space, centered around `[8, 8]`, with y increasing downward. Closed webs must follow clockwise screen order and enclose positive area. Adjacent vertices must differ. |
| `orientations` | Required angle per lane, each from -256 to 256. A full turn is 256 units; negative turns are clockwise. These orient the rolling claw and enemies. |
| `start_lane` | Required zero-based starting lane index. |
| `depth` | Required tunnel depth, 1–1000. |

Open webs support 1–128 lanes; closed webs support 3–128. Tankers need at least
three lanes, and Alien Droids need at least two.

### `[flippers]`, required

This table is required even when ordinary Flipper generation is disabled.

| Field | Meaning |
| --- | --- |
| `enabled` | Generate ordinary Flippers. Defaults to `true`. If `false`, at least one other generator table must exist. |
| `count` | Legacy name for the shared wave budget. Required unless top-level `enemy_count` is set. |
| `first_spawn_ticks` | Required initial spawn counter, 1–65535. |
| `spawn_interval_ticks` | Required repeat spawn counter, 1–65535. |
| `climb_speed` | Required depth units per tick, from 1/65536 to 16. |
| `flip_speed` | Required rolling speed, 1–64 angle units per tick. |
| `pause_ticks` | Required pause counter between flips, 0–255. |
| `max_active` | Required positive active-Flipper limit. |

### Optional enemy generators

| Table | Enemy |
| --- | --- |
| `[tankers]` | Flipper Tankers. |
| `[fuse_tankers]` | Fuseball Tankers. Requires `[fuseballs]` for released children. |
| `[spikers]` | Spikers that build spikes. |
| `[beasts]` | Armored Beasts. |
| `[fuseballs]` | Fuseballs that travel along lane boundaries. |
| `[super_flippers]` | Super Flippers. |
| `[spiral_flippers]` | Spiral Flippers. |
| `[mirrors]` | Mirror Flippers. |
| `[extra_spikers]` | Additional Spiker generator. Requires `[spikers]` with the same `climb_speed`. |
| `[aliens]` | Alien Droids. |

Each table enables its generator and requires `first_spawn_ticks`,
`spawn_interval_ticks`, and `climb_speed`, with the same ranges as the Flipper
fields. Optional `super_chance` and `spiral_chance` default to 0 and accept
0–255, expressing probabilities out of 256. Their effects depend on the
generator; see the bundled levels for combinations used by the original game.

### `[pulsars]` and `[pulsar_tankers]`, optional

Each requires `first_spawn_ticks`, `spawn_interval_ticks`, and `climb_speed`,
plus `phase_ticks` and `propagation_ticks`, each 1–255, for the Pulsar cycle and
lane-to-lane pulse timing. They do not accept `super_chance` or `spiral_chance`.
When both tables exist, their phase and propagation values must match.

### `[appearance]`, optional

| Field | Meaning and default |
| --- | --- |
| `surface` | Tunnel surface RGB, each channel 0.0–1.0. Default `[0.0, 0.0, 1.0]`. |
| `surface_hues` | Optional Jaguar CRY hue values, 0–255, one per lane. An empty list hides tunnel surfaces. Omit it to use `surface`. |
| `lines` | Web line RGB, each channel 0.0–1.0. Default `[0.015, 0.01, 0.78]`. |
| `flipper` | Flipper RGB, each channel an integer 0–255. Default `[240, 0, 0]`. |

### `[powerups]`, optional

`kill_interval` is the number of kills between power-up drops, 1–255, default 5.
The first kill drops a power-up, followed by one every `kill_interval` kills.
The power-up sequence itself follows the game's normal rules.

The bundled files in [`levels/`](../../levels/) provide more complex examples.
The [reference guide](../../docs/reference.md#editable-levels) describes the
original level sets and mechanics.
