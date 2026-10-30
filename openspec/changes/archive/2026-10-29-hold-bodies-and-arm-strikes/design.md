## Context

The Force powers' classes (`thaum::power::force`) and their live systems (`kinesis_live`, `throw_live`) already steer bodies; the force-shell aura already draws skinned copies (`effect_draw`). This change adds the body-side state the original's routines put on held bodies and weapons. No docs/formats/ page is involved (no file format).

## Decisions

- The held environment is the body's, not each part's: Exarust's `Ragdoll` integrates all its parts with one environment, so the original's per-node swap of the environment pointer becomes one field on the body.
- Releases go through a message (`HoldEnd`) written by the caster tick and applied right after it, because the caster tick reads the caster's body while the release writes another body.
- Rigid aura copies are children of their source meshes (they follow the node with no per-frame work); the meshes' box is fitted once, in the body's first part's frame, when the copies are made.
- The strikes program is a shader definition on the shell material (the same vertex stage, `effectshell_v`, is shared in the original).

## Open questions and how they were answered

- What is the special "material" Kinesis and Throw put on held nodes? REA: a simulation environment object built from an eight-word record (0.125, 0.0125, 0.025, gravity 0, 0, 0, 40, 20) converted for 83 ticks; the same conversion as the default environment's.
- What is the node field the shell's texture scale divides by? The component map's node layout: the bounding radius.
- What does the strikes per-frame setup pass? REA of the strikes geometry: `LclMat` identity for a rigid node, `FdPln` (0, 1, 0, centre_y − half_y) times plane scale over half_y, `Gradient` from the builder, four `ColPt`, `Fade` 1 − shown.
- Not answered: the reader of the simulation flag set on held nodes (not reproduced).
