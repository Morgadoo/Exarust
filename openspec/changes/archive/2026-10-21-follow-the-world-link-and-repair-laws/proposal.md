# Proposal

## Why

The second pass over the rigid-body unit (world links, soft weld, the
inverted-tetrahedron repair and the particle history) added a second
world-link system beside the one the root bodies already used, a
breakable-grab arm beside the grab pull, and a repair that kept an
inverted shape instead of rebuilding it. A review against the original's
machine code found the link laws, their float order and the repair
differing from the original in several places.

Phase 3 (Bodies: skeleton, physics, control), the rigid-body milestone of
`fit-shape-builders-and-the-dynamics-world`.

## What Changes

- One world-link type: the duplicate module and its tests are removed;
  `WorldLink` carries the original's branch order in centimetres
  (horizontal drag, slider, damped spring, plane, plain or slack), the
  disabled and last-pass skips and the slider's doubled push.
- Soft welds take a quarter of the strength times the gap less 0.4 of the
  two points' motion since the tick began, shared by 0.1/(m + m').
- The limp slack link keeps the original's sums and its quiet compare on a
  NaN length change; the slack reset tries the slack first.
- The inverted-tetrahedron repair rebuilds as the original does: current
  points := start points shifted by the box centre's displacement, the
  affine from the rest basis, axes re-formed at dot < 0.1 in the order
  (1,2), (2,0), (0,1), particles put back on the rest tetrahedron.
- Draw interpolation sums a + (b − a)·t as the original does.
- Docs say what is wired: root bodies' stored links become world links.

## Out of scope

- Driving doors, levers and plates (their steps) and creating world links
  for them; they stay static level geometry.
- The group-mobility blend into a child, the parent-disabled grab removal,
  sound sources of the scrape and roll loops.
- The draw fraction read from the node clock (Bevy's overstep fraction
  stands in).

## Evidence

REA (Ghidra 12.1.4) on the rigid-body unit read against its assembly: the
link iteration, the repair, the translate, weighted push and slack reset
routines and the draw interpolation; the vector helpers (length, weighted
add) read from the executable. Hand-worked numbers in
`body_repair_test.rs`.
