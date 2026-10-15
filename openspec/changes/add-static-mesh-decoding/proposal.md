# Proposal

## Why

Nothing in the world can be shown until static meshes decode. Mesh decoding is
the first item of Phase 1 in `docs/PLAN.md` and leads to milestone M1, "asset
viewer". Every prop, level piece and weapon part uses the same mesh chunk, so
this change unblocks Phase 2 (static world).

## What Changes

- Decode static meshes from chunk-tree files: name, transform, vertex streams
  (positions, texture coordinates, and normals where present), polygon groups
  with their material names, triangulated for rendering.
- Convert units and axes once, to metres and Y-up.
- Add `exa-tool export-gltf <pack> <entry> <out.glb>` so a decoded mesh can be
  checked in Blender, with textures resolved by name from the texture packs.
- Add `exa-tool corpus meshes` to report how many chunk trees decode, per pack.

Evidence sources:
- corpus statistics over Objlib, Objects, Components and Resource;
- the original mesh reader in `Exanima.exe`, examined through REA (Ghidra provider);
- visual comparison of exports against the running game.

Out of scope:
- skinned meshes, weights and morph targets (Characters, `*base.rfc`, `*morph.rfc`);
- level grids and placement;
- material parameters beyond texture slot names;
- collision bodies (read but not interpreted);
- Bevy loaders (Phase 2).

## Capabilities

### New Capabilities
- `mesh-decoding`: turning static mesh chunks of the original data into triangle meshes with materials references, in Exarust's units and axes.

### Modified Capabilities
- `asset-inspection-cli`: adds glTF export and a mesh corpus report.

## Impact

- `crates/exa_formats`: new `rfc` mesh module.
- `crates/exa_tool`: new `export-gltf` and `corpus` commands; new dependency on a glTF writer (or a hand-written GLB writer).
- `docs/formats/RFC.md`: mesh section moves from "observed" to "verified".
- `docs/PLAN.md`: Phase 1 progress.
