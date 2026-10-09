# Starfield reference tables

`rantab.bin` is the 512-byte big-endian expansion of the original
`src/dat/rantab.dat` assembly data. Gameplay's `siney` indexes this table to
choose the second sine frequency, including its special level-32 lookup.
The signed sine bytes are shared with `assets/warp/sines.bin`.

Patterns follow `initstarfield`, `siney`/`initpstarfield`, `circa`, and
`ringstars` in `src/yak.s`. The gameplay `fields` table contains `siney` in
both slots, so the normal background varies by level rather than alternating
between random and patterned fields.
