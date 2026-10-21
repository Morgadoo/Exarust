# Tasks

- [x] Verify every rule of the ten routines against the assembly and their call sites; correct the crystal id compare, the row walk, the glow's ramp and hold, the latch's consumer, the strip's two records and the power-down's trigger.
- [x] Drive the effects' scan from the reconstruction device on the game clock, test for it as the original does, and scale the device's voxel lights by the glow.
- [x] Check the installed `objdb.rdb` for the shipped teleporter (`0x555`, class 2 kind 15, `PrtlSmblA.rfi`, destinations 7, 10, 12, 13) and reconstruction device (`0x556`, class 2 kind 18) with `exa-tool`; no test reads game data.
- [x] Update `docs/formats/OTHER.md` and the crate READMEs; `docs/PLAN.md` is updated by the main session.
- [x] `make check` (remote: `cargo test -p exa_world -p exa_game`, `cargo clippy -p exa_world -p exa_game --all-targets -- -D warnings`).
