# Proposal

## Why

The original smooths meshes at run time: its character-creation scene
subdivides the hair it shows, a debug command subdivides a whole scene, and
the round effect shells (bubble, force bubble, force barrier) are subdivided
polyhedra. Behind them sit the mesh tools that turn a mesh into its
editable form and back into render form (smoothing groups, seam and
mirror splits, a vertex-cache order). None of it exists in Exarust:
`RFE_MeshSubdivide` and `RF_Polyhedra` are at 0%, `RFE_MeshTools` at 25%
and `RF_Mesh` at 20%.

## What Changes

- Decoded meshes keep each vertex's texture-coordinate element and the
  per-triangle edge bits (`0x3DC2`), so the editable form can be rebuilt.
- `exa_assets::mesh_edit`: the editable form (channels of corner indices,
  streams, the incidence of each position element in the original's
  order), smoothing groups from triangle flags, and the conversion back to
  render form: runs by smoothing group, material, texture seam (textured
  materials) and texture winding (normal-mapped materials), unified
  vertices numbered group by group.
- `exa_assets::vertex_cache`: the original's vertex-cache triangle order
  with its fast logarithm and exponential reproduced bit for bit, the
  flag sort and renumbering by first use.
- `exa_assets::mesh_smooth`: the original's Loop subdivision with its
  crease, seam and fold rules, weights, output order and face-bit copies,
  plus the per-node pass with the bounds refit.
- `exa_assets::polyhedra`: the icosahedron, its top cap and their
  subdivision for the effect shells.
- `ModelLoader::smoothed_model` and `exarust model … --msm`, the
  original's `msm` debug command for one model.

## Capabilities

### New Capabilities

- `mesh-editing`: editable form, render-form conversion, vertex-cache
  order, subdivision and polyhedra.

### Modified Capabilities

- `world-viewer`: the smoothed model view.

## Impact

- `exa_formats::rfc::Mesh` gains `uv_ids`, `uv_count` and `triangle_edges`;
  `TextureSlot::flag`.
- New modules in `exa_assets`; `ModelLoader::smoothed_model`; `--msm`.
- Docs: `docs/formats/RFC.md` (Editable meshes), `docs/RENDERING.md`
  (character views, Effect shells), `docs/DEBUGGING.md`, the crate README.
- Evidence: REA on the mesh-smoothing unit (its main pass, crease marking,
  seam bits, position and texture edge and vertex points, face split, the
  per-node and per-scene passes and the debug command), the mesh tools
  (incidence, smoothing groups, run splits, tangent split, render-form
  conversion, vertex-cache order, merge sort, quicksort, score tables, fast
  logarithm and exponential), the mesh helpers (revert, smoothing channel
  removal and rebuild, render-data drop), the bounds refit and the
  polyhedra constructors and subdivider; corpus: `CharExt.rpk` (12 hairs,
  24 meshes, every position and texture element referenced, smoothing
  bits in the triangle flags).
- Out of scope: hair itself (the CharExt loader, the hair material and the
  head attachment), the item-preview and scene-wide passes, the effects
  that draw the shells, the UV sphere of the mind effects, welding and
  winding flips, the GI voxeliser wrappers in the smoothing unit.

## Phase

Phase 5, visual fidelity: run-time mesh smoothing and the effect shells'
geometry.
