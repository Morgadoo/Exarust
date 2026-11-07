# Proposal

## Why

The original fades the roofs, awnings, overhangs and upper floors that
stand between the camera and the player. Exarust modelled the detector and
the fade step (`exa_world::scene_nodes`) but nothing called them, the mesh
callbacks stopped at the bounding sphere, and the draw of a fading mesh
was unknown: the captured shader library has no main-pass fade, so the
previous assessment guessed a dithered discard. Occluders were drawn solid.

PLAN.md Phase 5 (visual fidelity), the scene-node unit and the player's
camera.

Evidence (REA, `Exanima.exe`): the camera's per-frame cut-away driver and
its segment, the scene probe (kinds, carries), the static mesh's segment
callback (a swept 50 cm sphere) and ray callback, the mesh loader's
parent-carry rule and fade target, the view's draw sequence (the faded
pass after the blended surfaces: depth only, then colour with the
material's `MatDif.a` replaced by one less the fade and a source-alpha
blend), the material bind's blend-factor table, and the supersampled
draw of a settled mesh (half of the jittered sub-frames). A corpus test
counts the shipped candidates.

## What Changes

- `exa_formats::rfc::Node`: the cut-away bit names.
- `exa_assets`: `ModelNode::cut_away` (`CutAwayMesh`: the loader's bits and
  the triangles the callbacks walk).
- `exa_world::cut_away`: the `CutAway` record attached at spawn (with the
  parent and children carries resolved), `sphere_cast` and `ray_cast`;
  `scene_nodes::fade_step` split out of the fade pass; the model detector
  asks the callbacks; a mesh's fade target defaults to ½.
- `exa_game::cut_away_live`: the live detector, fade and draw; the chase
  camera publishes its segment.
- `exa_render`: `ExaParams::cut_away_opacity` (the free slot), `flags2`
  bits for the blended and the settled supersampled draws, a pipeline key
  that writes depth for a blended copy.

## Out of scope

The depth-first pass itself, the per-material opacity of a material's
first faded mesh, the alpha test of an alpha-tested surface while it
fades, the drawn-sector test (Exarust asks the parts' view visibility), the
grid set pieces' carries, the dissolve effect's dithered shadow caster
(state bit 4, a different feature).

## Impact

`exa_formats`, `exa_assets`, `exa_world`, `exa_render`, `exa_game` and the
`ModelNode` literals in other crates; docs/RENDERING.md, docs/formats/RFC.md,
the crate READMEs. Rendering only: `--fixed-frames` traces are unchanged.
