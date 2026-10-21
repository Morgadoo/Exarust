# Proposal

## Why

The original's navigation unit holds the camera's arithmetic (the align
wrap, orbit, pitch floor, zoom, frame clock, placement, preset, kick and
the cut-away probe and its driver), the screen picks (plane picks, the
nearest-mesh pick, the kinesis pick) and the Poisson-disc scatter the
terrain grass uses. Exarust had only the screen ray, the plane pick and the
chase camera's own copies of a few rules. A first port (DeepSeek, branch
`oc/navigation`) added the unit but misread several rules: the yaw wrap's
reference, the preset's default retention (a double read as a float), the
kick's basis, the cut-away segment's side and its candidate bit, the
pick's bounds test and the kinesis lift, and it swapped the grass
scatter's sine and cosine.

## What Changes

- `exa_game::nav_camera`: the align wrap about the camera's present yaw,
  the engine's planar yaw, the orbit rate in double precision, the pitch
  floor's precision and the held pitch, the free zoom per wheel unit, the
  placement from the unclamped point, the preset record (with the double
  3.94e-13 retention), the cut-away segment beyond the eye and the kick
  with its own direction.
- `exa_game::nav_pick`: the nearest-mesh pick with the segment against the
  bounding sphere and the box, the class-word no-pick bit, the ground-plane
  fallback, and the kinesis pick with the grown sphere and the crossing of
  its top; the world cursor uses a stand-in node for the physics world.
- `exa_world::scene_nodes`: the cut-away probe over the drawn sectors with
  the scene word's candidate and kind bits, the overhead probe and the
  parent and children carry.
- `exa_world::terrain::vegetation`: the scatter's x is the cosine, from the
  engine's sine and cosine.

## Capabilities

### New Capabilities

### Modified Capabilities

- `player-movement`: camera navigation rules; navigation picks.
- `scene-nodes`: cut-away probe.
- `terrain`: vegetation scatter offsets.

## Impact

- `crates/exa_game/src/{nav_camera,nav_pick,player,cursor}.rs`,
  `crates/exa_world/src/{scene_nodes,terrain/vegetation}.rs`,
  `docs/PLAYER.md` (Camera, cursor pick), crate READMEs.
- Evidence: REA/Ghidra assembly of the unit's routines (the evidence pack
  `context/re/packs/navigation/`), and of the engine helpers they call
  (planar angle, arc cosine and sine, sign, MINSS/MAXSS wrappers, the
  segment-sphere and segment-box tests, the sine-cosine routine and its
  constant table), plus the data words they read.
- Out of scope: the ground pick, the editor drag trio, the viewport
  detach, the gamepad zoom, the zoom-to-pitch chain, the stepped zoom's
  per-millisecond easing, the drag-start snapshot, the preset's GUI
  callbacks and editor scan, the jittered offset table, and putting the
  cut-away probe, the preset and the placement on the draw path (the scene
  model is not drawn yet). Mesh callbacks stay stand-ins (no CPU
  triangles). No retail capture of the camera or the grass points.

## Phase

Phase 3, bodies and control: the player camera and cursor pick (M34's pick
under the cursor), with the grass scatter of Phase 2's terrain.
