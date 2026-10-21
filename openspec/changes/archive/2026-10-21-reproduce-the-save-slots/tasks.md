# Tasks

- [x] Package placement, opening and the sentinel, with synthetic packages laid out by hand.
- [x] The 52-byte headers, the character placement, masks, journal, world database sections, start check and arena conversion.
- [x] Slot names, scan verdicts, order, new slots, deletion, loads, checked saves, checkpoints, permadeath, legacy removal and the transition save, on temporary folders.
- [x] The game clock and the death threshold.
- [x] The session save in the package shape; saving over a retail-shaped save stays readable by a model of the original's reader.
- [x] Real data: the executable names the sections (skipped without `EXANIMA_DIR`); a folder of real saves round-trips byte for byte when `EXANIMA_SAVES` is set (two retail saves were checked in place).
- [x] Update `docs/formats/SAVES.md`, `docs/formats/OTHER.md`, `docs/formats/README.md` and both crate READMEs.
- [x] Hand the `docs/PLAN.md` phase note to the integrating session (this lane does not edit `docs/PLAN.md`).
- [x] `make check` equivalent: `cargo fmt --all`, clippy `-D warnings` and the tests of `exa_formats` and `exa_game` on the build worker.
