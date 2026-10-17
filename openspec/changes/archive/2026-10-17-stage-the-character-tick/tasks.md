## 1. Research

- [x] 1.1 Re-read the sector step and the body tick in `exe.asm`/REA: the character list from its last entry down, 20 iterations with the pair pass first, the pair list dropped, detection in the 3×3 cells after the iterations, body–level detection, 8 friction passes with the pair friction first, the post-step; the body tick's calls in address order; findings appended to `context/re/master-plan.md` (M0 note) and `body-tick-order.md`

## 2. The tick's stages

- [x] 2.1 `exa_physics::body::stages`: `BodyPreStep` and `BodyPostStep` chained around `CharacterTicks` and `BodySystems`; the `CharacterTick` schedule (single threaded) with the nine `CharacterStage` sets chained; `TickedCharacter`; `run_character_ticks`
- [x] 2.2 `SolverStage` (one no-op method per stage), `SolverStages` (added in order, found by type), `solve_with` calling them at the original's points; `solve` is `solve_with` without stages; `step_bodies` passes the registered stages
- [x] 2.3 `exa_game::stages`: `CharacterTickPlugin`, the driver (living bodies with ticked clips, the player last, each with the controller above it, nothing before the motion set) and the posture, movement, sampler and carry modules; `limp` keeps the rules they call; the sampled pose passes in `SampledPose`
- [x] 2.4 `exa_audio` split into `bank` (with `SoundSet` and `SoundBank::pick_from`), `music`, `hits`, `footsteps`, the picks drawn in the same order as before
- [x] 2.5 `exarust --fixed-frames` (a debug helper): every frame advances game time by exactly 1/60 s, so runs repeat tick for tick and traces compare between builds

## 3. Tests

- [x] 3.1 The solver calls its stages in the original's order (20 iterations numbered 1–20, pair pass before and links after every body's links, contacts held through the iterations and dropped before detection, 8 friction passes on this tick's contacts, post-step last); a stage with no work changes nothing over 120 ticks; stages are found by type
- [x] 3.2 Each character runs all nine stages in order before the next; the tick's sets run pre-step, character ticks, solver, post-step
- [x] 3.3 The driver ticks living bodies with ticked clips, the player last, each with its controller through a plain node; a limp body and a frame-played body do not tick; nothing ticks without the motion set; the player is ticked last
- [x] 3.4 Sound sets name variants from 01; the shared picker stays in its set and draws the same first variant as before; levels pick their music; the installed `Sound.rpk` holds every variant of every set and no more (skipped without `EXANIMA_DIR`)
- [x] 3.5 The existing body tests pass unchanged (clips sampled every tick at four frame rates, a walking controller identical at any frame rate, the whole capture)
- [x] 3.6 A temporary probe (removed): the staged tick against the old monolithic one, three bodies walking, turning and shoved, 400 ticks at 60 and 47 FPS: every particle, target and clock identical

## 4. Verification

- [x] 4.1 Real installation (skipped without `EXANIMA_DIR`; no data format changes, so no `exa-tool` corpus command applies): `--fixed-frames --trace` of `--shove 60 play arena` (light runs through `gpu_run.sh`), the build before this change (with the flag) against this one: of the runs whose body was built on the same tick (25) with the blow on the same tick (135), all 5 751 rows (blow, Defence, Fall Backward, lying, Get Up 2, a minute standing) are identical in every column but the body's entity id
- [x] 4.2 Remove probes and dumps from the tracked tree

## 5. Documentation

- [x] 5.1 `docs/formats/RSQ.md` (Physical body: how a tick runs), `docs/ARCHITECTURE.md` (schedules), `docs/DEBUGGING.md` (`--fixed-frames`), `docs/PLAN.md` (Phase 3, Phase 5 audio), READMEs of `exa_physics`, `exa_game`, `exa_audio`
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` (fmt, clippy -D warnings, tests; `play` profile)
