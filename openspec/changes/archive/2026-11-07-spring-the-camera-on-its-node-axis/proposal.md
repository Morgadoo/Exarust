## Why

PLAN Phase 3 (player control and camera). The navcam gap lane rebuilt the
routine table of the game camera's controller (TNavCameraControl, 63 %).
Its 17 ms tick turned the view with a scalar yaw spring and eased the zoom
linearly, while the original springs the backward vector toward the camera
node's axis and runs the zoom on its capped spring; the wall probe ran
without an answerer and eased its tilt at a guessed rate of 1.

## Evidence

REA (Exanima.exe), read in the assembly: the controller's tick (the zoom
spring or snap, the shake, the pitch ease, then the heading: the shown yaw
is the planar yaw of the backward vector, the target yaw and its twin turn
2nπ in double when the heading jumps past π, the node is turned to the
pitch plus tilt and the target yaw led at most the lead field, and the
vector springs to the node's third row by the view stiffness and the
retention, normalised by the reciprocal root past 1e-8); the frame's
placement (the field of view from the spring's last two values, the shown
zoom kept past 0.01 in double); the placement routine (vector, shown and
target yaw, spring values and the wall fields reset); the wall-avoidance
routine (the ease scaled by the zoom-spring rate field) and its scene walk
(kind 0x5 meshes asked the segment callback from the far end reversed,
others the ray callback from the start). Corpus: 171 level meshes (arena7,
arenao1, arenao2, arenao4, exanima08, exanima67) and the object library's
trees and buildings carry both probe bits.

## What Changes

- exa_game: `ChaseCamera` keeps the backward vector and the zoom spring;
  `nav_camera` gains `wrap_target_yaw`, `lead_yaw`, `spring_back`,
  `zoom_settles`, `frame_zoom`, `frame_zoom_kept`; the preset's misnamed
  pitch speed is the zoom step; the wall tilt eases at the zoom-spring rate;
  the chase camera answers the wall probe from the drawn obstacles.
- exa_assets: `CutAwayMesh::probe` keeps the obstacles' triangles.
- exa_world: `cut_away::WallObstacle` attached by the model spawner, with
  the ray and swept-sphere callbacks; corpus test of the obstacles.
- docs/PLAYER.md (Camera), crate READMEs.

## Out of scope

The free-flight branch of the tick (pan keys, terrain lift, focus tether,
area bounds), which moves only the unfollowed looked-at point; the
node-driven branch of the editor drags; the grid tileset loader's own
probe mark; the per-sector visible and seen bits (stood in for by the
parts' view visibility); the blocking word.
