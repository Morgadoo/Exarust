# Proposal

## Why

Phase 3 (milestone M3, "standing man") needs a controllable character inside
an original level, with collision, as the base for the physics body and
combat.

## What Changes

- `exa_physics`: fixed 120 Hz rapier world, static triangle-mesh colliders
  from level geometry, kinematic capsule characters (slopes, steps, gravity,
  ground snapping).
- `exa_game::player`: movement relative to a chase camera (WASD, Shift to
  run), the body turning to its heading, Idle/Walk/Run clips by speed.
- `exarust play <level>` (with `--autowalk` for unattended checks); a stand-in
  start cell until level start markers are decoded.
- `docs/research/openpbr.md`: how OpenPBR fits the renderer (material model;
  Original vs Enhanced shading), linked from Phase 5 of the plan.

Out of scope: the articulated, motor-driven body; combat stance and
mouse-directed attacks; original start positions; dynamic props.

## Capabilities

### New Capabilities
- `player-movement`: moving a player character through an original level with collision and matching clips.

### Modified Capabilities

## Impact

`crates/exa_physics`, `crates/exa_game`, `crates/exa_world` (`spawn_point`),
`crates/exa_assets` (meshes kept in the main world for colliders),
`crates/exarust`, docs.
