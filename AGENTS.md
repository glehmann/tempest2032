# Project guidance

- The goal is to make Tempest 2000 easy to install, preserve its exact gameplay,
  and provide graphics for modern hardware. Treat gameplay fidelity as a goal,
  not a claim that every existing implementation is exact.
- Preserve original mechanics and 60 Hz simulation timing when changing
  presentation. Keep deliberate gameplay options separate from normal mode.
- Consult [docs/reference.md](docs/reference.md) for source references, timing
  quirks, and deliberate differences from the original before changing them.
  Module documentation and focused tests provide additional context.
- Keep the README short and player-facing. Put technical explanations in the
  reference guide or next to the relevant code, and update them with changes.
- System ordering lives in `src/app.rs`; level definitions live in `levels/`.
  Keep standalone builds self-contained with embedded assets and levels.
- For Rust changes, run `cargo fmt --check`, `cargo check`, relevant tests, and
  `cargo clippy -- -D warnings`. Use the capture tools described in the reference
  guide to check visual changes when possible.
