## Why

Roadmap task B4, lane `collision3`: the open rows of the port queue's lane
Q19 (collision and queries). The leg snap's static mode (the Height Query
over the sectors' static meshes only) was ported but nothing asked it: the
story continue's exit nudge put the player by the living placement, and the
summoned demon kept the place its build gave it. The Ground Query met every
body mesh its box gather found, without the original's slab pre-test of the
body node's box. The cloth's sphere contact took the brute-force query's
push floor on every level mesh, also those the original answers through
their kd-tree.

## Evidence

REA (Exanima.exe 0.9.5): the exit nudge (the nudged point handed to the
pathfinder's nearest walkable point, written into the character frame, then
the placement in mode 1 and the held re-place); the demon's arrival (the
frame at the summon's place and turn, the placement in mode 1); the Ground
Query's body loop and its segment/box pre-test (approximate reciprocal of
the direction, entry clamped at 0 before the exit, entry before the
length); the mesh's sphere query in its tree-leaf and brute-force forms.

## What changes

- `exa_game::limp::GroundPlacement` gains `at`, the frame's place for a
  caller that moved the character in the same step.
- `exarust` viewer `place_player_in_step`: a nudged continue takes the
  nearest walkable point and asks the static ground placement.
- `exa_game::thaum::summon_live::arrive_demons` asks the static ground
  placement at the summon's place and facing.
- `exa_physics::intersect::segment_box_slabs` and `Level::floor_box`; the
  Ground Query pre-tests a rigid body's stand-in by it.
- `exa_physics::cloth_level::LevelPiece::tree` and the floor argument of
  `sphere_push`; `ObjectContacts::gather_level` takes the piece's form.
