# Proposal

## Why

docs/PLAN.md, Phase 3 (Bodies: skeleton, physics, control), the
`reproduce-the-navigation-unit` paragraph: "Not in: the ground pick, the
editor drag, the gamepad zoom, the stepped zoom's per-millisecond easing,
CPU mesh triangles for the picks". The navigation unit's remaining
routines (ground pick, free-camera absolute drag and its start, vertical
zoom slide, zoom pitch offset, stepped wheel zoom, editor grab drag, the
preset's distance clamp, handler set and editor scan, the viewport
attachment, the stored-point translate and the jittered offset table) had
no Exarust counterpart, and the live wheel used a rounded signum instead of
the unit's stepped routine.

## What Changes

- `exa_game::nav_pick::ground_hit` (with `screen_ray`, `Ray::unit`,
  `PlaneHit::world_distance`): the ground pick's grown sphere (5 cm when
  the caller accepts the node, 1 mm when not), 40 m reach, ask from the
  grown top (or from the eye with the whole reach when the eye is under
  it), last answering node kept, ground-plane fallback.
- `exa_game::nav_drag`: `zoom_detents` (now the live wheel in
  `player::chase_camera`), `zoom_slide`, `zoom_pitch_offset`,
  `pitch_with_offset`, `FreeDrag`/`free_drag_start`/`free_drag_release`,
  `GrabDrag`/`grab_start`/`grab_target`.
- `exa_game::nav_camera`: the preset's distance clamp and derive flag,
  `preset_handlers`, `scan_target`/`preset_target`, `Attachment`,
  `translate_points`, `disc_offsets`; the kick's square sums in the
  original's order.
- Review fixes, each checked against the listing: the ground pick's grow
  values are centimetres (0.05/0.001 m, not 5/0.1 m) and a negative
  crossing is zeroed; the zoom slide squares before multiplying into the
  range; the grab drag builds the looked-at point from the raised grab
  point and normalises before its planar yaw; the release snap tests the
  rounded count's low 32 bits and the pixel bounds wrap; the editor scan
  keeps all three half extents; a NaN preset distance derives; the kick
  sums `z² + (y² + x²)`; a touchpad's pixel scroll counts 24 pixels to the
  notch in the live wheel.

## Evidence

The navigation pack's listing (assembly of the ground pick, the drag,
slide, pitch-offset, wheel, grab and preset routines) read instruction by
instruction; the offset table's group counts and start indices read from
the executable's data (1, 2, 3, 4, 8, 15, 24, 66 at 0, 1, 3, 6, 10, 18,
33, 57).

## Out of scope

CPU mesh triangles (the picks still stop at the `RayMesh` trait), live
callers for the ground, kinesis and nearest picks, the absolute and grab
drags (no editor viewport), the handler installation itself, the
cut-away's per-mesh callbacks, the Steam Input poll.
