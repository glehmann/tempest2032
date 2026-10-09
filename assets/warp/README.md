# Gate-flight surface

`beasty6.cry` is the original 320×128 big-endian Jaguar CRY texture referenced
as `pic4` by `m7test` in Tempest 2000's `src/yak.s`. It was copied from
`src/images/beasty6.cry` in the reference source tree.

The texture is embedded in the executable and decoded using `src/cry.rs`.
No reference-source directory or asset conversion is needed at runtime.

`crosshair.cry` is the 40×39 sprite at (250, 110) in the original 320-pixel-wide
`beasty4.cry` sprite sheet, copied by `m7scr` in `yak.s`. Zero CRY words are transparent.

The original sine table now lives in `src/sines.rs`. It is only 256 bytes and
callers need exact integer values, so embedding it in Rust source beats a
binary asset.
