## 1. Measure

- [x] 1.1 Three runs of each golden scene on one binary: pixel noise per shot
- [x] 1.2 A frame digest of each frame's picture inputs, compared between runs
- [x] 1.3 F567: the scene run 225 times (cores on, gdb, load, the crashing binary)

## 2. Wire

- [x] 2.1 Lockstep readback and exact shots offscreen (`--fixed-frames`, `EXA_OFFSCREEN_SHOTS`)
- [x] 2.2 Synchronous pipeline compilation under `--fixed-frames`
- [x] 2.3 Background jobs waited for and frame budgets lifted under `--fixed-frames` (`exa_physics::job_wait`)
- [x] 2.4 `--frame-digest`
- [x] 2.5 The last frame waits for pipeline compiles and the GPU (`exa_render::exit_wait`)

## 3. Prove

- [x] 3.1 Three runs of each golden scene with pixel noise near zero; the golden noise allowance lowered
