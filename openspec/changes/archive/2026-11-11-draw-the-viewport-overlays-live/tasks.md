## 1. Find
- [x] 1.1 REA: the viewport draw slot's overlay tail and the form layout's inner right and bottom, in the assembly
- [x] 1.2 REA: every writer of the frame-rate, version and title flags and the ShowFPS setter
- [x] 1.3 REA: the camera's height offset target field and its writers

## 2. Port
- [x] 2.1 `view_overlay`: flags, readout counter, places, shadow and inks, one rasterised node
- [x] 2.2 The flow's title return and session start and the splash's hand-over set the flags
- [x] 2.3 `fps_text` in the engine's formatting; the old readout removed
- [x] 2.4 Tests; docs/UI.md, docs/PLAYER.md and the crate READMEs

## 3. Check
- [x] 3.1 GPU grid against r189: exanima01 play and the title after the splash
- [x] 3.2 Lockstep traces against r189 and the determinism grid
- [x] 3.3 fmt, clippy -D warnings, tests, play build, openspec --strict
