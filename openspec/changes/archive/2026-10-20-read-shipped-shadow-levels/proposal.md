# Proposal

## Why

In exanima01 and other dungeon levels the ground around broken-floor
patches showed soft dark bands, as if lit through holes or around raised
geometry. The cached cube shadows were correct to the original's rule: a
level-3 floor or terrain receiver is shadowed by a level-0 caster up to
about 1 % of the light distance behind it. The broken-floor slabs
(`uw01_floorbrk*`) lie 1–2 cm under the painted dirt, so the terrain over
them read as shadowed. Exarust gave every mesh outside the grid sets level
0, but meshes ship their own level: the word before a mesh body's streams,
which the parser called skinning flags, is the node's render word, and its
bits 16–18 are the shadow-bias level. The broken floors ship level 2, at
which a caster must be about 1 % in front of a receiver to shadow it.

## What Changes

- `exa_formats::rfc::Mesh::render_flags` (was `skin_flags`), with
  `shadow_bias_level()` (bits 16–18) and `shadow_bias_inflated()` (bit 19).
- Model nodes carry the level; the spawner tags each mesh part with it.
  Grid sets and terrain keep setting their own levels.
- The bit-19 variant (both sides × (1 + 0.005·v)) is not reproduced: such a
  mesh keeps level 0 (one shipped mesh).
- Debug tools: `exarust --debug-column x,z[/x,z…]` logs the meshes over a
  floor point; `exa-tool corpus render-words` tallies the render flags.

## Capabilities

### Modified Capabilities

- `rendering`: shadow-bias levels come from the mesh when no builder sets
  one.

## Impact

- `exa_formats::rfc`, `exa_assets::model`, `exa_world` (spawn), the
  `ModelNode` constructors.
- `exarust::column`, `exa_tool::corpus`.
- `docs/RENDERING.md` (Filter), `docs/formats/RFC.md` (Mesh body),
  `docs/DEBUGGING.md` (Surfaces and shadows).
- Evidence (REA/disassembly of `Exanima.exe`): the mesh-body reader copies
  the word into the node's render word when the chunk version is at least
  0x3D22; the depth-scale routine reads v = (word >> 16) & 7; the grid-set
  marker and terrain constructor overwrite it with their own levels.
  Corpus: 3 317 meshes carry a level, every `uw01_floorbrk*` level 2.
  Notes: `context/re/optimizations.md` §8.2, `context/re/shadows.md` §3.
- Out of scope: the bit-19 variant, the cull-front bit (`0x800`, unused by
  any shipped mesh) and the keep-own-light bit (`0x1000`).

## Phase

Phase 5, visual fidelity: shadows.
