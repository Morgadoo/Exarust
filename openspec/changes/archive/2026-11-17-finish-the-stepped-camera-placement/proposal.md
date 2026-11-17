## Why

Roadmap task B4, lane L12 (camera and navigation). The camera's placement
routine, which every scripted shot and every preset (the camera's
construction and each game mode's record) runs, has a tail on the stepped
camera that Exarust left out: the zoom moves to its nearest stop and the
pitch is held at once. Exarust's camera started at zoom 1300, between the
stops, where the retail camera sits at 1280 (its live field of view is
29.93° at 1600 × 1080, the 1280 zoom's). The fast shots also clear the
pitch the orbit drag asked for, the arena's spectated start is a fast
two-point shot, and the free camera pans with the bound movement keys,
not fixed W, S, A, D.

## Evidence

REA (Exanima.exe 0.9.5), instructions read:

- The placement: the distance raised to the smallest; on the stepped
  camera lowered to the largest, then the wheel's routine with no turn
  (stop level rounded, 0 to 5, back to a zoom; floor from that zoom; held
  pitch the steeper of floor and asked pitch; target pitch minus it), then
  the placement's pitch held between the held pitch and the cap, written
  to the target and the shown pitch.
- The preset applier runs the placement (with the record's distance), so
  the constructor does.
- The point and two-point shots zero the asked pitch when fast; the story
  shot does not. A shot without a distance passes the controller's
  current distance.
- The zoom handlers derive the floor from the target zoom; the camera
  tick does not recompute the pitch target.
- The arena's match starts call the two-point shot fast with the spawn
  spot and the spot plus 700, 1180, 1100 cm.
- The free branch's keys are four globals that the controls' table
  rebuilds from the first four binding slots.
- Retail capture (2026-10-30): the live camera's field of view 29.93° at
  1600 × 1080.

## What Changes

- `nav_camera::fixed_placement` and `STEPPED_CAMERA`; `ChaseCamera::face_shot`
  runs the stepped tail and keeps the placement's pitch as the target until
  a zoom or orbit handler runs; `take_shot` clears the asked pitch for a
  fast shot; a new camera runs the placement.
- The pitch floor follows the target zoom.
- `CameraFrame::clears_pitch`.
- The spectated arena start goes through `frame_two_points` (fast) with its
  pointer warp.
- The free camera's pan keys come from `Bindings`.

## Impact

- `exa_game` (`nav_camera`, `player`, `flow`, `camera_modes_test`).
- On screen: every play view's zoom is 1280 at the start (about 1.5 %
  narrower than before); shots snap the pitch; the spectated arena start
  begins at the start zoom.
- Lockstep traces unchanged (camera only; the frame side).
- Out of scope: the cursor pin's saved view the shots also refresh (the
  cursor's own routine, not ported), the player's own and bracket arena
  round shots, the slow-motion follower, the detach on a switch to no mode.
