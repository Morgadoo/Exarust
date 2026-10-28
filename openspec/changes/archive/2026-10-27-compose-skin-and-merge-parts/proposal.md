# Proposal

## Why

Second pass over the skinning and character-texture units (docs/PLAN.md
Phase 5, character surface; Phase 3 bodies for the skinned mesh). The
original composes each character's skin maps with three generated programs
and the beard overlays, paints a garment's run-time materials from a
single job stem, merges the body and garments into one skinned mesh, and
matches garment seam groups with the enclosing part's group stack.
Exarust had the race maps, per-part stems, the per-node mesh and the plain
apparel matching.

## What Changes

- `exa_assets::skin_composite`: `chargen_d`, `chargen_s`, `chargen_n` and
  the beard's colour, normal and surface overlays per texel; the beard is
  drawn over the map's quarter corner only; `chargen_n` binds at most five
  layers, stopping at the first amount not above zero. Not fed to the
  renderer.
- `exa_character::texture_job::passes`: the callback's draw order (colour,
  normal, surface, each followed by its beard overlay).
- `exa_assets::model::painted_model_stem` (wired): a garment is painted
  from one stem, the first `&` material of a depth-first walk, and every
  `&` part takes it.
- `exa_assets::merge`: material runs selected by index range, per-vertex
  cull masks, the renumber pass with back-to-back runs, sub-object ranges;
  `model::scene_merged_mesh` merges a scene's skinned groups. Not wired.
- `exa_assets::skin`: the sixteen-frame group stack and the apparel
  matching's parent context, where parent points only raise the bar. Not
  wired; the model build passes no parents.
- `factory::preview_side`, `model::underscore_skinned_child`,
  `model::skinned_mesh_under`.

## Evidence

The units' assembly: the `&` walk (parts, then children, one stem flag
shared over the walk), the render callback (target order, clear colours,
the overlay quad spanning 0..0.25 with full texture coordinates, the layer
loop's unordered compare and five-layer cap), the overlay program sources,
the append and renumber passes (inclusive last vertex), and the apparel
matching (stack points lower the bar without being chosen).

## Out of scope

Feeding the composite to the skin material, the merged mesh as the draw
path, collecting the group stack in the model build, the layer strength
and jitter, the item preview job, and the controller and scene hooks Bevy
provides.
