# `exa_character`

Characters: races, body morphs, appearance, apparel fitting, stats/attributes/skills, health and injury state.

**Phase:** Phase 3–4 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

## Scope

- Races (`races.rdb`), roles (`charroles.rdb`), body morphs and hair (`CharExt.rpk`).
- Apparel layering (`Characters.rpk` meshes fitted to the body, `Apparel.rpk` textures).
- Attributes, skills, experience; health, stamina, injuries per body part.

## Apparel

- `Outfit` lists garments by `Characters.rpk` model; `equip_outfits` fits
  each to the body parts, paints it from its appearance (or from the first
  object-database item worn as the model) and hides the body parts it
  covers. The fitting (the garment's weights from the nearest body vertex,
  its meshes) and the painting run on the async compute pool, and at most
  32 MiB of painted textures are made a frame; a body is dressed, 8 ms of
  dressing a frame, once everything it wears is in, before the animation
  systems bind the skinned parts in the same frame.
- Underwear: humans marked `WearsUnderwear` (the player, the character
  view's human body) wear the object database's braies (item 8) beneath
  their outfit whenever nothing in it covers the groin on the innermost
  layer (`needs_underwear`), as the original keeps its human records.
  Placed characters are dressed exactly as stored. See
  [`docs/formats/OTHER.md`](../../docs/formats/OTHER.md) (Underwear).
