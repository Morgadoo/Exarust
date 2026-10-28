# Proposal

## Why

docs/PLAN.md, Phase 3 (characters), the character generator and skinning
paragraphs: the dressed-mesh plan (`exa_character::dress`), the `_SKIN`
merge (`exa_assets::merge`) and the skin composite
(`exa_assets::skin_composite`) were reviewed, tested rules that the running
game did not use. It drew one mesh per scene node, and the skin used the
race maps with a tone tint and the first normal map.

## What Changes

- Each dressed character whose body model numbers its slots is drawn as
  the original draws it: one merged skinned mesh built from the dressed-mesh
  plan through `merge::merge`. Each material run is a sub-mesh bound to the
  character's skeleton. The body nodes it replaces, and the body slots a
  garment covers, are hidden. Held items, hair and body cloth keep their own
  draw. A body the slot passes cannot plan (the demons) keeps the per-node
  draw.
- The merged skin run takes the character's composited skin (the
  `chargen_d`/`_s`/`_n` programs, the beard in the face corner, at most five
  normal layers). It is generated on the async compute pool, once per
  distinct look, and the run draws the race maps until it is ready.
- Review fixes: the merged frames are composed in the file's row-vector
  convention (the lane had turned every rotation the other way, leaving the
  arms raised in the bind pose); each run keeps the splitter's tangents
  (without them no normal map shaded); the race maps are read unshifted,
  because the look seed's `UVOfs` belongs to the decomposition programs
  only; the composite no longer runs on the main thread.

## Evidence

The `chargen_d`, `chargen_s` and `chargen_n` loaders bind only their
texture units and colour uniform. `UVOfs` is looked up only by the
decomposition unit's loaders (`chardecomp_d`, `_s`, `_n` and its seam
program). The file's frames place a point at `p.x·r0 + p.y·r1 + p.z·r2 + t`,
as `transform_from_rows` builds the Bevy transform. The GPU run on the RTX
3090 (`play exanima01`, `--opponent play arena1`, `character Default/Idle`)
matches the integration branch's look, and the dressing stage takes as long
as it did before.

## Out of scope

- Per-triangle cull bits and the occlusion mark: the built meshes keep no
  file flags, and no shipped material sets the mark.
- Hair as cloth, off-hand items, the feral darkening in the shader, the
  look rebuild's re-dress, and the inventory preview's skin job.
- `pose::rows_of`'s own convention, which the saved-pose module uses
  consistently with its own maps.

## Phase

Phase 3 (characters): the character generator's dressed mesh and the skin
compositor on the running path.
