## Why

The original's chase camera follows the player's pelvis node, interpolated
to the frame between its body ticks, so the view bobs with the walking body
by the pelvis's 1–3 cm. Its controller ticks every 17 ms and every frame
shows its backward vector and zoom between its last two ticks. Exarust's
camera follows the capsule's feet and shows its last 17 ms tick as it is,
so the view steps at 59 Hz on faster screens and ignores the body.

Phase 3 (Bodies: skeleton, physics, control), milestone M3 "standing man";
the master plan's lane D change M3 (MS2 standing man).

## What Changes

- **Focus** (`exa_game::player`): the camera looks at the player body's
  pelvis (the root part), drawn between its last two body ticks exactly as
  its mesh is drawn, or at the player's feet while it has no body.
- **Ticks**: a tick runs once the frame passes its start (the original's
  tick time ahead of the frame), at most nine a frame, the rest of a
  backlog dropped.
- **Interpolation**: each frame shows the backward vector (renormalised),
  the zoom (and so the field of view) and a shake offset between the last
  two ticks, by how far the frame is into the last tick.
- **Shake**: `ChaseCamera::kick(v)` adds v (metres) to the eye's shake
  offset and takes it from the tick before's, as the original's shake call
  does; each tick carries the offset on (0.96 of its change, then 0.99 of
  the whole; zero once both are tiny), a damped wobble of about a second.
  It moves the eye only, and only above the original's 1e-5 cm². Nothing
  kicks the camera yet.
- Docs: `docs/PLAYER.md` (Camera), `docs/PLAN.md` (Phase 3), the
  `exa_game` README.

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5 (REA): the camera frame driver
  (17 ms steps while the tick time is behind the frame, at most 9, then
  snapped to the frame; t = 1 − (tick time − now)/17), the per-frame
  placement (follow node interpolated through its body, backward vector
  and zoom lerped and renormalised, fixed 2000 cm distance, FOV from the
  zoom every frame, shake offset lerped and added to the eye above
  1e-5), the camera tick carrying the shake offset on and the shake call
  that kicks it, start of play setting the follow object to the player's
  `01Pelvis` (`context/re/master-movement.md` §11, `player-control.md` §2,
  `master-movement.md` §19).
- Exarust in game through `gpu_run.sh` (light run, `--autowalk --trace` in
  the arena): the camera's focus is the drawn pelvis in every frame, its
  height the traced pelvis height between the frame's two ticks within
  0.0003 cm, rising and falling 99.7–102.1 cm with the walk.

Out of scope:
- the wall tilt and the occluder fade (M9, render work);
- shake sources (RB30: hits, falls; M9, M30, C22);
- the camera's own body-relative zoom modes and cursor look.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `player-movement`: the chase camera follows the pelvis and shows its
  last two ticks blended; its tick rule; its shake (kick and ring-down).

## Impact

- `exa_game` (`player.rs`: `ChaseCamera`, `chase_camera`, `camera_focus`)
- `docs/PLAYER.md`, `docs/PLAN.md`, `crates/exa_game/README.md`
