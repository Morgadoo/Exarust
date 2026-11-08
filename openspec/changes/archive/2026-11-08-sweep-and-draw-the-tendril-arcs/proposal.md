## Why

Phase 7 in docs/PLAN.md (thaumaturgy in play), component-gap lane effects (round 146):
TTendrilEmitter tested a character's limbs with their collision spheres and an untraced
20 cm value, and drew the Shock power's and shock weapons' arcs with an additive glow
stand-in instead of the electricity program.

## Evidence

REA on `Exanima.exe` 0.9.5:

- The emitter's body test runs once per limb body (limb flags 0x12000000, a character owner,
  not the hand's owner). It takes each tendril segment, from the first point to two short of
  the end, into the limb's frame (the inverse of the limb's matrix, the hand's offset carried)
  and fills the engine's swept-sphere record: length, start, a radius of 20.0, end, unit
  direction and the reciprocal direction. That is the record the body-level step builds
  for a moving sphere, so the 20 cm is the swept sphere's radius. It hands the record to the
  limb geometry's swept query (the mesh's own callback, or its kd-tree's) with a callback
  that only sets the emitter's hit byte. The first segment that sets it shocks and branches.
- Every arc node the electricity unit creates takes the material `_ElecMat01`, a 240-byte
  record in the executable: emissive (1, 1, 1), opacity 1, blend word 0x161, shading flags
  0x480018, one emissive layer `BeamNoise01` (in `Textures.rpk`).
- The electricity program (shipped GLSL, already `arc.wesl`) is bound with EleCol
  (0.65, 0.4, 1) and EdgCol (0.15, 0.2, 0.4); the draw uploads TexOfs = (0, the emitter's
  shown scroll).

## What Changes

- `exa_physics::body::pairs::swept_touches`: a sphere swept through a part's mesh (its
  kd-tree, its triangles, or its spheres when it has no mesh), hits only noted.
- The live arcs' limb test uses it with the 20 cm radius instead of segment-to-sphere
  distances.
- The live arcs draw with `exa_render::electricity::ArcMaterial` and `BeamNoise01`, each arc
  with its own texture offset.
- `arc.wesl` returns an alpha of 0: the pipeline blends `AlphaMode::Add` premultiplied, and
  the alpha of 1 it returned blacked out what lay behind the ribbon (no arc had drawn with
  it live before).
