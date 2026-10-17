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

## Held items

- `held::Hands` says what a body holds as the original's moveset rule reads
  it (the main item's weapon flags and whether it is a light, a second
  weapon, a shield, an off-hand light); `held::choose` turns it into a
  `HeldMoveset`: the moveset, the stance class and the weapon class (the
  fists `0x80`). Dressing puts a wielded weapon in the main hand, the game
  crate puts a lit torch in its hand, and a system after dressing writes
  the chosen moveset into the body's `Animator` whenever the hands change.
  Only bodies with `Hands` follow it (the player and the duellist). See
  [`docs/PLAYER.md`](../../docs/PLAYER.md) (Moveset from held items).
- A wielded weapon's `Blade` carries its record as the original builds it
  (`exa_formats::weapon_record`): a plain model measured along its
  first-child chain of rigid parts, a factory weapon up to its assembled
  top, held by the character's own size, scale and encumbrance (its
  fighter's stats, `CharacterStats::holder`; an average human without a
  fighter) and its race (`RaceWeaponry`: the race index and `races.rdb`
  weapon fields, the human's by default). See
  [`docs/formats/OTHER.md`](../../docs/formats/OTHER.md) (Weapon record).
- An unarmed fighter's hands are its fists: dressing gives each hand a
  `Blade` (the right the first held item, the left the second) from the
  race's fist or claw records, held as its weapons are, a segment from the
  hand's origin across its box (17.66 cm for a human), and leaves the
  fighter without a primary
  striker; a wielded weapon is the primary from the start. See
  [`docs/PLAYER.md`](../../docs/PLAYER.md) (Combat mode).

## Blood

- `blood::PaintWound` paints a wound now, as the original's painter does:
  around the contact point less half the striking point's travel, a ball
  behind and a smear ahead, into the vertex colour's alpha (255 clean,
  only ever lowered) of the character's skinned meshes and the meshes under
  its head, on their current skinned positions. Held items, rigid items on
  other parts and hidden parts are never painted.
- A model's meshes stay shared until a wound lowers a value of one: the
  struck character then gets its own copy (marked `Bloodied`), painted in
  place afterwards. A whole human body in braies costs 278 KiB, the
  duellist's outfit 1.9 MiB, on the CPU and the GPU each.
- `blood::DebugWounds` (`--debug-wound`, [`docs/DEBUGGING.md`](../../docs/DEBUGGING.md))
  paints test wounds once the player's or the character view's body is
  dressed.
- Wounds from hits: each `exa_combat::wounds::Bleed` (a blow's wound whose
  seven ticks are up) is painted as a `PaintWound`, on corpses too.
- Blood on weapons: a wielded weapon's holder carries its
  `exa_combat::wounds::BladeBlood` and a `blood::WeaponStain` (its tip's
  height and the meshes the original stains: the first mesh of the model's
  root node, or every part of an assembled weapon). Once the blood passes
  0.01, `stain_weapons` adds a stain pass (`exa_render::stain`) over each
  of those meshes and keeps its offset with the blood; the gradient
  `BldGr02D` loads the first time a weapon that can be stained exists.
  `blood::DebugWeaponBlood` (`--weapon-blood`) gives the player's weapon
  blood from the start.
- Blood sprays (`spray`): each `exa_combat::wounds::Spray` starts a `Jet`
  in the scene's one `BloodSprays` (a resource): fixed to the struck part's
  frame (where the body's last tick left it, else where the frame draws
  it), against the swing, sending its droplets at 40 a second from one
  shared emission clock. The sprays tick in the fixed step after the body
  solver (droplets age, fall at a quarter of gravity, keep 0.95 of their
  speed a second; never touch anything) and are drawn each frame as
  camera-facing diamonds in one world-space mesh with
  `exa_render::spray::SprayMaterial`, between the last two ticks; the
  droplet texture `BldPrt02` loads with the first jet. `spray::DebugSprays`
  (`--debug-spray`) fires test jets from the player's or the character
  view's body every second. See
  [`docs/RENDERING.md`](../../docs/RENDERING.md) (Blood sprays).
