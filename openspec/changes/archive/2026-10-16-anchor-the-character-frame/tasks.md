## 1. The frame

- [x] 1.1 Re-read the anchor routine, its body-tick, get-up and locomotion callers in `exe.asm`, and read its constants with REA `read_bytes` (5.0, 10.0, 0.0, 0.95 double, 0.25, 0.4 double); notes in `context/re/body-drive.md` §15
- [x] 1.2 `exa_physics::body` (`body_frame.rs`): `follow` (the rule table: get-up 10 cm in the first quarter then none, down 5 cm, moving onto the pelvis, no support 0, standing 5 cm under the double 0.95 else none, height kept standing) and `anchored` (pelvis less the root offset at the pelvis's height, three-axis distance in cm over the reach; first time at once)
- [x] 1.3 `Ragdoll::anchor_frame` at `aim`/`hold`, after the carry: the anchor, the frame (the root's turn at the anchor), every target moved across the ground with it; `Drive::anchor`, `Drive::frame`
- [x] 1.4 The carry carries the anchor and the frame; `Ragdoll::followed` takes the controller's follow out of the next carry
- [x] 1.5 `hold_on_frame` (was `hold_on_controller`): toward the frame's pelvis, only damping while moving; `STANCE_HOLD`
- [x] 1.6 `Bearing::rising`, set by `exa_game::limp` from the get-up clip's clock (first quarter of its last key, last tick's clock)
- [x] 1.7 `CharacterBody::follow` / `followed`: each step, before the walk, a tenth of the way, at most 2 cm, through the controller; `exa_game::limp` sets it from the anchor after the aim and tells the body what was followed before it
- [x] 1.8 `exa_game::balance::get_up` turns the body, no longer moves the controller
- [x] 1.9 The body trace's `frame_x`, `frame_z`, `root_x`, `root_z`

## 2. Tests

- [x] 2.1 The rule table pinned: 10 / 5 / 0 cm, 0.3, 0.95 as a double (0.95f follows, the next float stays), the height kept standing
- [x] 2.2 The placement: at once the first time, 5 cm stays and 5.01 follows, the height counted lying and brought standing, onto the pelvis moving
- [x] 2.3 A standing humanoid pushed at 3 m/s: at support 1 the frame stays and the body rests within 1 cm of its spot; at 0.95 the frame follows and the body rests no more than 5 cm back from where it went (14 cm), at its frame's pelvis
- [x] 2.4 The carry leaves the follow out (30 cm root move: carried, or not when followed)
- [x] 2.5 A get-up re-anchors: 30 cm at once, 8 cm stays, 12 cm follows, past the first quarter stays
- [x] 2.6 The hold pulls across the ground only and only brakes while moving
- [x] 2.7 The controller glides after its follow point: 2 cm a step from 50 cm to 20 cm, then a tenth, nothing within a millimetre or unset
- [x] 2.8 The get-up's first quarter (`exa_game::limp`); the clip test and the trace test read the targets on the frame (placed under the body by the first tick)
- [x] 2.9 Corpus checks (skipped without `EXANIMA_DIR`): the shipped body's joints and legs, unchanged
- [x] 2.10 A steady frame 90 m from the world's origin stays exactly put for 600 ticks (the anchor is carried relative to the root; through the root's inverse it crept 5.1 mm)

## 3. Verification

- [x] 3.1 In game through `gpu_run.sh` with `--trace`: R1 `play arena` standing (the frame frozen while support ≥ 0.95); R2 scripted quarter turns and combat steps; R3 `--shove 60` (the controller follows the fall and the get-up at ≤ 2 cm a tick, no jumps; stands where it fell); R4 `--opponent`; `--autowalk` arena (walk speed, the frame on the pelvis); no validation errors from the change
- [x] 3.2 Remove probes and dumps from the tracked tree
- [ ] 3.3 R5 `play exanima01`: not run, the GPU ran out of memory loading it beside the user's retail game (four tries, the lightest settings included); to rerun when the GPU is free

## 4. Documentation

- [x] 4.1 `docs/formats/RSQ.md` (Physical body: the character frame, the controller's follow, the stance hold, the get-up, the trace columns), `docs/PLAYER.md`, `docs/PLAN.md` (Phase 3), `crates/exa_physics/README.md`, `crates/exa_game/README.md`
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 `make check` (fmt, clippy -D warnings, tests; `play` profile)
