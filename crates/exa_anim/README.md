# `exa_anim`

Skeletons and motion sequences from `.rsq`, keyframe controllers, pose blending; produces targets for the physics character.

**Phase:** Phase 3 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

## Scope

- Parse `humanoid.rsq`: bone hierarchy (18 bones for humans) and motion sequences.
- Keyframe controllers (linear / Catmull-Rom), pose sampling and blending.
- Procedural layers: stance, aim of the weapon from mouse input, look-at, idle noise.
