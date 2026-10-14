## Why

Exanima's bodies are physical: the dead lie where they fell and a knocked-out
fighter collapses under the blow. Exarust only played a canned fall clip, and
the levels' corpses stood in their placement pose.

## What Changes

- Decode the physical body chunk of `humanoid.rsq` bones: links to the parent
  part (pivots and range limiters), foot contacts and thorax pins.
- Add the original's body simulation without muscles. Each part is a rigid
  four-particle tetrahedron; links are hard radii; sphere contacts with
  friction against the static world; Verlet integration at 83 Hz.
- Bodies asked to go limp become ragdolls from their current pose and stop
  being animated and steered.
- Placed corpses go limp where they lie. A fighter whose health runs out
  goes limp instead of playing the fall clip.
- Viewer: `--limp` drops the character in the character view; `--at x,y,z`
  overrides the level start.

## Capabilities

### New Capabilities
- `physical-body`: limp simulated bodies built from the skeleton's body data.

### Modified Capabilities
- `combat`: knock-downs make the body go limp.

## Impact

- `exa_formats::rsq`, the new `exa_physics::body`, `exa_game::limp`,
  `exa_combat`, the viewer, and `docs/formats/RSQ.md`.
