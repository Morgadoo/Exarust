# Proposal

## Why

Fourth pass over the skinning and character-texture units (docs/PLAN.md
Phase 5, character surface; Phase 3 bodies for the skinned mesh). The
merged mesh's culling append, the skin joint capture and the texture
callback's decomposition branch did not follow the original: the cull
bits went to the body slots, the joint capture walked breadth first, and
the decomposition passes read the wrong units and were never drawn.

Evidence: the original's culling append (per-triangle flag word tested
against the cull bits), the generator's per-garment slot loop that calls
it with the slot's cull bits over the garment's visible mask and its body
loop that calls the plain append, the recursive joint capture (frame set
only where none is, children visited only then), and the texture
callback's decomposition branch (mask pass into an R8 target, its two
material textures, the mask rebound at units 1 and 5, `SCol` passed as
`SknCol`) read in the objdump listing; the shipped mask program's GLSL;
the shipped garments' flags (1 091 drawn groups of 54 models marked, none
with `0x400000` on a slot whose two cull masks are zero).

## What Changes

- `exa_character::dress` (wired): garment slots take the cull bits again;
  the body's own slots take none. `exa_assets::merge` drops the garment
  triangles whose per-triangle flags meet them, and a part without flags
  takes the unset word `1` once the merged mesh has a flag stream; the
  loader carries each built group's flags (`ModelNode::part_flags`).
- `exa_anim::skin::capture_joints` (wired into `bind_skins`): depth first
  in child order, the first node of a bone number capturing it and a
  repeated bone ending its branch.
- `exa_assets::decomp::mask` and `skin_composite::decomposition_mask`:
  the mask pass; `compose_with_decomp` feeds the race's colour or gloss at
  `Tex0`, the mask at `Tex1`/`Tex5`, the decay maps in the material's
  order and the tone as `SknCol`; a fifth normal layer reads the mask.
- `SkinCompositeJob::decomp` (wired): the job's feral level above zero
  with the seed offsets, in the cache key; the loader decodes the eight
  maps and draws the pass.

## Out of scope

- The original's crash path when a culling append meets a mesh without a
  flag stream (it appends the mesh plainly, then reads the missing
  stream); Exarust appends such a part whole.
- The `_SKIN` scene import's node flags, the model skin flag and the
  skinning manager's hooks (Bevy stands in).
- The callback's apparel layer draws onto the three targets.
