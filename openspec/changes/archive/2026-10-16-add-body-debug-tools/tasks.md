## 1. Measures

- [x] 1.1 `exa_physics::body::inspect` (`src/body_inspect.rs`): `joint_angles` (twist about the bone axis, then swing; atan2 angles), `angle_between`, `stretch` bands (1 mm tolerance, 1 cm), `bone_name`, `Sleep::name`
- [x] 1.2 `Ragdoll`: `with_rest_turns`/`rest_turn` (each part's rest turn, stored by `exa_game::limp` from the model's rest pose), `relative_turn`, `bone_axis`, `joint` (angles, clip error, clip turn), `link_report`, `worst_link`, `worst_link_of_body`, `range_used`, `origin_pivot`, `drawn_particles`, `drawn_link`, `drawn_anchor`
- [x] 1.3 `DealtShoves`: the blows each tick dealt, filled by `step_bodies`
- [x] 1.4 Unit tests: rest pose 0 (identity and non-identity rest); twist about the bone; swing across it either way (axis sign), half turn; swing·twist split and quaternion sign; angles between turns; stretch bands at their edges and NaN; a hip's joint, links, range and pivot; a living hip's clip error, anchor, drawn link and particles

## 2. Overlay and readout

- [x] 2.1 `exa_game::body_overlay`: `BodyOverlay` (F3, `--debug-bodies`), F4 only the inspected body, F5 pick under the cursor (scaled for the supersampled camera) or the next body; `InspectedBody` defaults to the player's
- [x] 2.2 Gizmos over the scene: thin tetrahedra and particles (`BodyFrameGizmos`), thick links coloured by stretch, pivots as crosses, slack spheres past the slack, faint anchors, a dot over the inspected head
- [x] 2.3 Readout (Bevy UI, default monospace font): body line, simulation line, a row per part by bone (total, swing, twist, swing axis, clip, range, pivot, past slack), coloured by the worst link
- [x] 2.4 Test: stretch colours

## 3. Joint trace

- [x] 3.1 `exa_game::joint_trace`: `--joint-trace`, 33 columns, a row per part per tick for bodies not in deep sleep, flushed each second
- [x] 3.2 Tests: header; a full row and an empty root row; a ticking body's rows per part per tick
- [x] 3.3 `--trace` gains `countdown`, `sleep` at the end; its test checks them

## 4. Simulation controls

- [x] 4.1 `exa_game::sim_control`: P pause/resume (`Time<Virtual>`), `.` one tick (timestep added to the fixed overstep before the loop), `[`/`]` speeds ×0.1–×2, `--sim-speed`
- [x] 4.2 `--pause-at-tick`, `--pause-on-stretch` (edge-triggered per body, reason logged and shown), pausing at the end of the tick
- [x] 4.3 Chase and fly cameras on real time; `--screenshot` counts from a scripted pause
- [x] 4.4 Tests: speed stops; pause at tick 40 with 30 FPS frames, two steps, resume; a quarter-speed run; the stretch edge trigger
- [x] 4.5 `--press key@frame,...` and a fixed play camera with `--camera`; parsing test

## 5. Verification

- [x] 5.1 No data format is parsed anew; the shipped skeleton is exercised by the in-game runs (every part of the human body measured)
- [x] 5.2 In game through `gpu_run.sh --gpu 4000`, no `Validation Error` in any log:
  - `--debug-bodies --opponent play arena` (side `--camera`): both fighters drawn over their meshes, every link green while they stand
  - `--debug-bodies --limp character`: the body asleep on the floor, its slack links held at 98–118 % of their range (limp damping keeps them at the slack, +1–2 mm), pivots 0.2–2 mm
  - `--opponent --joint-trace` for a minute (143 262 rows, ticks 76–5660, ten blows on the player, who goes limp at tick 2128): during "Head Pain" the clip flexes the player's elbows 94–97° from rest while the forearms stay 16–17° from it (89–97° off), the elbow pivots open 14 mm and the shoulder links reach 285–312 % of their slack; the opponent's right upper arm is left 84° from its clip's turn when "Cmbt - Idle" follows an attack, and its right elbow leaves its hinge link's slack (131 %) in "Attack Mid"
  - `--press F5@60,F4@61` with the cursor off the window: the opponent is inspected and drawn alone
  - `--pause-at-tick 300 --press .@500,.@510,.@520 character`: the trace ends at tick 303, the readout says paused at 303
  - `--pause-on-stretch 1.2` on the duel: paused at tick 278 on the player's right elbow pivot, 1.2 cm open
  - frame time against the build without the tools (CPU-bound character view, 3000 frames, two runs each): 2.89–2.95 ms without, 2.90–3.04 ms off, 2.96–3.01 ms with the joint trace, 3.01–3.05 ms with the overlay's gizmos, 3.78–3.79 ms with the readout, 3.84–3.87 ms with everything; the arena duel (GPU-bound) shows no difference beyond its noise
- [x] 5.3 Probes and dumps removed from the tracked tree

## 6. Documentation

- [x] 6.1 `docs/DEBUGGING.md` (flags, keys, overlay, readout, angle definitions, trace columns), linked from `docs/INDEX.md`, `README.md`, `docs/PLAYER.md`
- [x] 6.2 `docs/formats/RSQ.md` (body trace columns), `docs/PLAN.md` (Phase 3), crate READMEs (`exa_physics`, `exa_game`, `exarust`)
- [x] 6.3 `openspec validate --all --strict`
- [x] 6.4 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
