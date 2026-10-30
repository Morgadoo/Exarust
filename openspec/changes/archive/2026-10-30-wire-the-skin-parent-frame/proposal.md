# Proposal

## Why

Third pass over the skinning and character-texture units (docs/PLAN.md
Phase 5, character surface; Phase 3 bodies for the skinned mesh). The
skin builder's recursive pass hands each part's group centres to the
parts below it, and a garment child's joint matching reads them; the
texture job converts the appearance block's condition and names its
masks in ways Exarust did not yet follow.

Evidence: the original's assembly for the per-node pass, the apparel
matching, the stack push and the two group-list cleanups, the effect
mesh lookup, the texture job's layer list, the inventory preview job and
the decomposition draw (REA and the objdump listing), checked against the
pack's pseudo-code only for structure.

## What Changes

- `exa_assets::model::scene_skins` (wired): the per-node build walks the
  tree from its roots and hands every child its parent part's group
  centres (`skin::ParentFrame`); a part that keeps a shipped skin stream,
  or finds no group, hands an empty frame.
- `exa_assets::skin`: the apparel matching carries each group centre into
  the parent's space through the child's node transform before comparing
  it with the parent centres; `GroupStack` keeps the limit rules (a push
  of any size refused at sixteen, the separate empty push unlimited).
- `exa_formats::objdb::Appearance::condition` (wired through the paint
  job): byte × 1/255 in double narrowed to float, damage 2/(2 − d) − 1 and
  dirt (d² + d)·0.5 in float.
- `exa_assets::paint::mask_texture` (wired): the mask suffix is one
  character, the byte plus `'0'`.
- `exa_assets::paint::jitters`: the four job offsets, shared by the
  factory tiling; `Tiling::apparel` (a labelled guess, not wired).
- `exa_assets::model::skinned_mesh_under`: direct children in reverse
  order first, then the subtrees in forward order.
- `exa_character::factory::PreviewJob` and `exa_assets::decomp::Draw`:
  the preview job's model gate and the decomposition draw's state (not
  wired).

## Out of scope

- The original's eight-centre frame capacity and the spill of extra
  centres into the next frame's slot.
- The GPU programs of the texture callback, the `_SKIN` scene import and
  the skinning manager's hooks (Bevy stands in).
- Applying the apparel jitters, the item preview's draw.
