## Why

A duel left the player's arm folded backwards, and nothing in Exarust
shows why: the body solver's parts, links and muscle anchors are
invisible, the joints' angles are nowhere measured, the body trace has
one row per body with the pelvis only, and the simulation cannot be
paused or slowed to watch a fold happen. Body work (the folded arm, the
balance and support work of C6, everything after it) needs the solver's
state on screen and in a file, and a way to stop at the tick where it
goes wrong.

Phase 3 (Bodies), milestone M3 "standing man": tooling for the body
controller plan, alongside C6. The PLAN's risk table already calls for a
tuning harness against the original's feel; this is its first part.

## What Changes

- **Body overlay** (`--debug-bodies`, F3 in any mode): every simulated
  body drawn over its mesh between ticks: each part's particles and
  tetrahedron, each link from its point on the part to its point on the
  parent coloured by how far it is past its slack (green within, 1 mm
  tolerance; yellow up to 1 cm past; red beyond, with its slack as a
  sphere), pivots as crosses, and a living body's muscle anchors, faint.
  F4 draws only the inspected body; F5 inspects the body under the
  cursor.
- **Joint readout** with the overlay: for the inspected body (the
  player's by default), each joint's turn from the model's rest pose,
  split into a swing of the bone axis and a twist about it, how far it is
  from the clip's turn, how much of its range its slack links use, how
  open its pivot is, and how far its worst link is past its slack, rows
  coloured as their links are; the body's sleep state, support, balance,
  energy and tone; the simulation's speed and pause.
- **Joint trace** (`--joint-trace <csv>`): a row per body part per tick
  with the same measures, the clip's turn split the same way, whether a
  blow landed on the part in the tick, and the body's sleep state. The
  body trace (`--trace`) keeps its columns and gains the sleep countdown
  and state at the end.
- **Simulation controls**: P pauses and resumes the fixed step (the
  controllers, the character ticks and the solver), `.` steps one tick,
  `[` and `]` step the speed through ×0.1–×2 (Bevy's virtual time);
  `--sim-speed`, `--pause-at-tick <n>` and `--pause-on-stretch <cm>` for
  scripted runs, after which `--screenshot` counts its frames. The chase
  and fly cameras run on real time, so a paused body can be orbited.
- **Scripted runs**: `--press key@frame,...` presses keys at frames after
  the load; in play mode `--camera` places a fixed camera instead of the
  chase camera, for unattended pictures of a duel.
- Measures in `exa_physics::body::inspect` (pure angle maths with unit
  tests: swing/twist, stretch bands; joint, link, range, pivot and anchor
  reports on `Ragdoll`), each part's rest turn stored when the body is
  built, and the blows each tick dealt (`DealtShoves`).
- Bevy's gizmo and default-font features are enabled for the overlay and
  the readout.

Evidence:
- The body solver's own state (no new claim about the original is made):
  the measures read the particles, links and anchors as the solver keeps
  them; the angle maths is checked by unit tests.
- Exarust runs through `gpu_run.sh --gpu 4000`: `--debug-bodies
  --opponent play arena`, `--debug-bodies --limp character`, a one-minute
  `--joint-trace` duel, `--pause-at-tick` with scripted steps,
  `--pause-on-stretch` on the duel, and frame rates with the tools off and
  on against the build without them.

Out of scope:
- fixing the folded arm (a separate change diagnoses and fixes it, using
  these tools);
- replaying recorded inputs, and comparing against captures of the
  original side by side;
- drawing the sphere colliders, level contacts and the character
  controller's capsule.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `world-viewer`: the body overlay and joint readout, the simulation
  controls with their keys and flags, and scripted key presses and a fixed
  play camera for unattended runs.
- `physical-body`: joint measurements (turn from the rest, swing and
  twist, clip error, link stretch, range, pivot), the joint trace, and the
  sleep columns of the body trace.

## Impact

- `exa_physics`: `body_inspect.rs` (new, `body::inspect`), `body.rs`
  (`Part::rest_turn`, `DealtShoves`, `step_bodies`)
- `exa_game`: `body_overlay.rs`, `joint_trace.rs`, `sim_control.rs` (new),
  `limp.rs` (rest turns), `trace.rs` (sleep columns), `player.rs` (chase
  camera on real time), `lib.rs`
- `exarust`: `main.rs` (flags), `viewer.rs` (fly camera on real time,
  `--press`, fixed play camera, screenshots after a scripted pause)
- `Cargo.toml` (Bevy features `bevy_gizmos`, `bevy_gizmos_render`,
  `default_font`)
- `docs/DEBUGGING.md` (new), `docs/INDEX.md`, `README.md`,
  `docs/PLAYER.md`, `docs/formats/RSQ.md`, `docs/PLAN.md`, crate READMEs
