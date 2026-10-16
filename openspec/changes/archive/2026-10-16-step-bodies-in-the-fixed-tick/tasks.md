## 1. The tick

- [x] 1.1 `exa_physics::body`: `BodyPlugin` with `CharacterTicks` and `BodySystems` chained in `FixedUpdate` after the character controllers; `solve` runs each phase over every awake body (integration with the muscles' pull, 20 iterations, contact reset and detection, 8 friction passes, post-step); `Ragdoll::tick` is `solve` with one body
- [x] 1.2 `ShoveQueue` filled before each frame's ticks (`begin_frame`, which also resets the frame's tick counts) and dealt by the solver before integrating; `BodyTicks` counts ticks
- [x] 1.3 The controller writes its simulated position into the transform during the tick; drawing overwrites it after the loop
- [x] 1.4 Remove `step_ragdolls`, its per-body accumulator and `MAX_TICKS`; balance runs in `Update` over the ticks of the frame

## 2. Clips and targets

- [x] 2.1 `exa_anim`: `Animator::advance` and `ClipPose` shared by `play_clips` and the character tick; `TickedClips` keeps frames off a physical body's clips
- [x] 2.2 `exa_game::limp`: the character tick (player last) advances the clip by one tick, samples it, sets the targets to the root's world transform times the clip pose, and carries the body; `TickedClips` is added with a living body and removed when it goes limp
- [x] 2.3 `Drive`: this tick's and last tick's targets replace `aimed`/`target_steps`; `aim(root, targets)` carries and aims, `hold(root)` carries and keeps the pose; a fall or a get-up starts afresh

## 3. Drawing and the trace

- [x] 3.1 Each part keeps its last two tick ends; `draw_bodies` interpolates them before transforms propagate, after every frame system that moves a character or a body root, and writes local transforms under the drawn parents
- [x] 3.2 `exa_game::trace` and `exarust --trace <file.csv>`: one row per tick per traced body in the probe's columns, cm and clip frames, empty where not simulated

## 4. Tests

- [x] 4.1 Identical particles after every tick at 30, 60, 144 and 240 FPS (a carried, driven, shoved body and a limp one on a floor far from the origin)
- [x] 4.2 A blow sent in a frame with no tick is dealt by the next; parts are drawn between their last two ticks under their drawn parents, also when a frame system moves and turns the body root; the world solver ticks each body as alone
- [x] 4.3 The clip clock and the targets per tick at four frame rates; frames leave ticked clips alone; the clock's loop and follow-up; the player is ticked last; the trace's rows and columns
- [x] 4.4 Existing tests ported to per-tick aims (driven pose, stagger, fallen bodies, turning far from the origin, kicking legs, carried body)
- [x] 4.5 A character walking and turning on a floor under the game's controller and physics plugins: the character and every particle identical after every tick at 30, 60, 144 and 240 FPS, and each part drawn where its last two ticks put it under the drawn character

## 5. Verification

- [x] 5.1 Real installation (skipped without `EXANIMA_DIR`; no data format changes, so no `exa-tool` corpus command applies): `exarust --trace` in `play arena`, one row per tick with no gaps, the clock advancing 60/83 of a frame per tick and the pelvis at the floor plus the clip root height
- [x] 5.2 In-game, against the build before this change: R1 (standing a minute in `play arena`), R3 (`--shove`: fall and get up), R5 (`play exanima01`: shoved bodies, corpses, autowalk over a step), R6 (`--limp character`), `character Default/Walk`, sword holding, and walking under `--autowalk` at about 46 FPS (no divisor of 83)
- [x] 5.3 R2 and R4 by scripted keys (a temporary probe, removed), against the build before this change: quarter turns standing, W, W+A and W+D circles, Shift run, S backpedal, Tab footwork; walking into the `--opponent` duellist in combat mode and attacking lands hits in both builds (23 and 22, all weak, no knockdown in either); the get-up after `--shove` is drawn without the one-frame 20 cm jump of the uncorrected build
- [ ] 5.4 The right button's walk to the cursor, zoom and orbit, and a knockdown in a duel need a person at the controls (blows are too weak to knock down in either build)
- [x] 5.5 Remove probes and dumps from the tracked tree

## 6. Documentation

- [x] 6.1 `docs/formats/RSQ.md` (Physical body: how a tick runs, the per-tick trace), `docs/ARCHITECTURE.md` (schedules), `docs/PLAN.md` (Phase 3), `docs/PLAYER.md` (unattended runs), `crates/exa_physics/README.md`
- [x] 6.2 `openspec validate --all --strict`
- [x] 6.3 `make check` (with the `play` profile)
