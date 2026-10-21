# Proposal

## Why

A first pass over the light, shadow, material and effect units reproduced
several of them wrongly: a base light intensity that changed existing
lighting, a second convex volume, a camera that overrode Bevy's planes, a
shadow-map size taken from the wrong place, a spline table one quad short
with an invented close and fade, an invented noise buffer, a particle UV
table whose second half was made up, and a blood-spray reset that really
belongs to the weapon-blood list. Each rule was checked against the
disassembly and corrected, so the new units can be trusted when they are
wired.

## What Changes

- Voxel light node: base and active intensity and radius, the flag word and
  the colour paths; the base light intensity of the existing GI is unchanged.
- Camera node data (70 degree default, near and far planes), held as data;
  Exarust's cameras keep Bevy's planes.
- Material constructor defaults, the built-in `_Default` record, and the
  per-scene material manager as a generic table; an unknown material name
  in the loader shades with one shared `_Default`.
- Shadow-map plumbing: allocator size tables and class pick, the separate
  draw-time cube sizing, the cube pool, caster cull face and programs, the
  directional view and orthographic matrices, the cube projection.
- Spline strip: 2 049-quad index table, vertex-index fade, extrapolated
  close, join modes, draw-state width, 32-byte vertex, line fragment rule.
- Particle unit: the 128-entry UV table derived from the eight symmetries.
- Voxeliser constants, the blood spray's scene-node names and flag words,
  and the Shock charge object (`thaum::energy::shock_power`).
- Removed: the invented noise buffer, the duplicate convex volume, the
  spray session reset and the spare emitter accessor.

## Capabilities

### New Capabilities

- `light-effect-units`: the reproduced light, camera, material, shadow,
  spline and particle units and the rules each keeps.

### Modified Capabilities

## Impact

- `exa_render` (`camera`, `gi`, `shadow_map`, `splines`), `exa_assets`
  (`material_manager`, `ModelLoader`), `exa_formats::material`,
  `exa_world` (`voxeliser`, `particles`, voxel light wiring),
  `exa_character::spray`, `exa_game::thaum::energy`; `docs/RENDERING.md`
  and the crate READMEs.
- Evidence: REA (Ghidra 12.1.4) on the original's camera, voxel light,
  material constructor and manager, shadow allocator and cube draw,
  spline tessellation and draw state, particle table loop and data,
  voxeliser constructor, blood spray builder and Shock creator; constants
  read from the executable's data. Tests use hand-worked values.
- Out of scope: driving the renderer from the shadow plumbing, a play
  camera that applies the node's planes, the material manager inside the
  loader, a skill tree that calls the splines, building the spray node, the
  weapon-blood reset in the game start, live Shock power objects and the
  line program as a pipeline.

## Phase

Phase 5, visual and audio fidelity: lights, shadows and effect units.
