# Design

## Decisions

- **rapier3d directly, not a Bevy plugin.** avian3d 0.7 targets Bevy 0.19;
  rapier3d has no Bevy dependency and shares glam 0.33 with Bevy, so a thin
  integration owns the step. The character solver stays ours.
- **Fixed 120 Hz** step in `FixedUpdate`; characters move before the world
  steps.
- **Colliders from rendered meshes**: meshes keep their main-world copy;
  colliders are built once after transform propagation for subtrees marked
  `StaticGeometry`.
- **Start cell**: the open, floored, non-special cell nearest the grid
  centre, until the level's start markers are decoded.

## Risks / Trade-offs

- Thousands of trimesh colliders cost memory; merging per room comes with
  mesh merging.
- The kinematic capsule is a stand-in; Exanima's bodies are articulated and
  physically driven.
