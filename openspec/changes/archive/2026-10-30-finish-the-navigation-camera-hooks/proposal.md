# Proposal

## Why

docs/PLAN.md, Phase 3 (Bodies: skeleton, physics, control), the navigation
unit paragraphs (`reproduce-the-navigation-unit`,
`reproduce-the-camera-drags-and-ground-pick`): the gamepad zoom, the
handler installation, the cut-away's mesh-children cascade and the
followed point of the cut-away driver were left out or modelled loosely,
and the preset's editor scan tested the class byte alone.

## What Changes

- `exa_world::scene_nodes::Scene::detect_cut_away` (new cut-away cascade requirement): a mark carries to the
  mesh children only from the overhead kind; a clear carries to the parent
  and the children from every kind.
- `exa_game::nav_camera::scan_target`: a node counts only when its class
  byte is 0xA9 and its class word's low twelve bits are 1; only such a
  node's 0xAE marker raises the flag.
- `exa_game::nav_camera::cut_away_focus` (with `FollowedFocus`): the
  probe's centre is the stored point, the followed object's own point or
  its position.
- `exa_game::nav_camera::Attachment::install` / `set_detached`: a preset
  sets the wheel slot, adds its press and move handlers and refills an
  empty saved wheel slot; a detach removes the free camera's press and
  move handlers and restores the saved wheel. `scene_editor::CameraSwitch`
  attaches and detaches it as its followed record comes and goes.
- `exa_game::nav_drag::gamepad_zoom`: the stepped zoom through
  `gamepad::zoom_step` (already live for the pad), then the pitch floor
  and the held pitch.
- Review: the lane's change of the ground and kinesis picks to record the
  whole reach was reverted (the mesh callbacks write the nearest distance
  back and return the face index plus one); the detach removes the free
  camera's handlers, not the editor's; the stick zoom reuses the
  gamepad's step instead of a copy.

## Evidence

The navigation pack's listing and the executable's disassembly: the
ground and kinesis picks reload the query's length after the mesh call,
and both mesh callbacks store a smaller distance there and return the
face index plus one; the detach routine's remove call matches the free
camera's drag-start and move handlers by code pointer; the preset's add
calls and its empty-slot wheel save; the cut-away probe's two cascade
branches and the children's class filter; the preset scan's byte and
twelve-bit word compares; the gamepad zoom routine's product order and
clamp branches.

## Out of scope

CPU mesh triangles and the callbacks' swept radius, per-node class words
on the live pick walk, the cut-away mesh callbacks, the frame driver's
follower branch, the GUI handler binding itself, the switch's preset
application (the record's preset fields are not carried), a live caller
for the cut-away probe.
