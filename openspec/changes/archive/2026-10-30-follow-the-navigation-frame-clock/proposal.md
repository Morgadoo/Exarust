# Proposal

## Why

docs/PLAN.md, Phase 3 (Bodies: skeleton, physics, control), the navigation
unit paragraphs (`reproduce-the-navigation-unit`,
`finish-the-navigation-camera-hooks`): the camera's frame clock was a
float countdown, the controller's constructor and destructor had no
counterpart, the preset ignored a hand-placed distance, the stepped
pitch clamp compared the wrong sign, the orbit release snap had no live
caller, and the grass scatter truncated its grid cells and drew in single
precision.

## What Changes

- `exa_game::nav_camera::FrameDriver`: the integer millisecond tick clock
  with unsigned 32-bit compares, at most nine 17 ms ticks, the backlog
  snapped to the frame, and the fraction `1 − (clock − frame)/17` from a
  signed 64-bit difference divided and subtracted in double; the chase
  camera drives it from its elapsed real time (the old `frame_step`
  removed).
- `nav_camera::NavController`: the constructor's 45° angle, back-links,
  hook and task priorities (0 and 7), the saved viewport callback (empty
  when the viewport had none) and the destructor that runs it only then.
- `nav_camera::apply_preset_to`: a set keep byte keeps the controller's
  distance (raised to the least) and is cleared.
- `nav_camera::clamp_fixed_pitch`: the look-down amount `−pitch` is held
  to the floor then the cap, a NaN kept.
- `nav_camera::Attachment::redraw`: the detach sets the controller's
  redraw byte; the camera tick clears it.
- `nav_drag::gamepad_zoom`: only a stepped camera moves.
- `nav_pick::plane_hit_in_frame` and `KdMesh`: the plane pick in a node's
  frame and the kd-tree as a pick mesh.
- `player::chase_camera`: releasing the orbit button applies
  `nav_drag::free_drag_release`.
- `exa_world::terrain::vegetation`: the engine's double unit draw, grid
  cells rounded to nearest even, the cell side from the original's 1/√2
  double, the grid side rounded to even.
- Review: the cell side's precision and constant, the node-frame point
  transform's sum order, the saved-callback slot and the redraw byte's
  role were corrected; the frame stamp wraps at 2³² ms.

## Evidence

The navigation pack's listing and the executable's disassembly: the frame
driver's add-and-compare loop (`jae` unsigned, nine-tick `jl`), its
zero-extended 64-bit subtraction, `divsd`/`subsd`/`cvtsd2ss`; the
constructor's angle constant and two list adds with priorities 0 and 7,
and the destructor's null test on the saved callback; the preset's keep
byte test and clear before the least-distance compare; the placement's
negate-compare-negate pitch clamp; the detach's byte write and the camera
tick's three clears of it, read by the frame driver's redraw branch; the
gamepad zoom's stepped flag test; the node-frame pick's point and vector
transforms; the scatter's `cvtss2sd`/`mulsd`/`cvtsd2ss` cell side, its
`cvtss2si` cell and side conversions, the unit draw's `cvtsi2sd`/`mulsd`
and the range delete's tail move.

## Out of scope

The Steam Input call in the frame driver, the GUI callback and task-list
plumbing (flags only), live callers for the mesh picks, the cut-away
detector on the draw path, the swept-sphere mesh callbacks, the 59th
root's source and the controller's timer resets.
