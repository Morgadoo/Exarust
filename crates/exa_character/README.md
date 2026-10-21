# `exa_character`

Characters: races, body morphs, appearance, apparel fitting, stats/attributes/skills, health and injury state.

**Phase:** Phase 3–4 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

## Scope

- Races (`races.rdb`), roles (`charroles.rdb`), body morphs and hair (`CharExt.rpk`).
- Apparel layering (`Characters.rpk` meshes fitted to the body, `Apparel.rpk` textures).
- Attributes, skills, experience; health, stamina, injuries per body part.
  The record and its rules are in [`record`](src/record.rs).

## Records

- `record::CharacterData` is a character's live record: the saved fields
  (`exa_formats::characters::CharacterRecord`) plus the reference count,
  the staged item list the game keeps beside the inventory and the one
  value the save copies out of the body. It starts blank (yellow and red
  full, focus 0), from the character creator's defaults (a male human of
  average physique, skin `0x8000`, hair `40 80 40 00`, pitch 1, in the
  underwear) or from a creator preset (its name cleared).
- Its rules are the original's character-data routines: the underwear rule
  (a human whose worn items do not cover the groin gets item 8 appended),
  the equip conflict pass (refused when an overlapping worn item's equip
  lock is over 50, as for the golem suit; otherwise worn items whose
  occupancy overlaps the new one on any bone come off, and the caller gets
  them back to drop or store), and the inventory and hand references
  (`mark_equipped`, `take_equipped`, `trim_hands`, `stow_hands`,
  `drop_surplus_refs`, `remove_item` (the last copy not equipped, the worn
  list through the game's skipping loop), `remove_everywhere`,
  `clear_items`, `strip` (returns the released references), `pick_up`,
  `add_worn`, `remove_worn`).
- Saved loadouts (four hand slots and worn items plus a word) copy to and
  from a record; applying one also ensures the underwear. Moving a record
  between databases remaps its item references (every id goes through the
  remap), or keeps one copy of an item that is both held and carried.
- Skills: the total levels (5000 points each), whether the record may
  spend another level (no skill holds a technique past the levels its
  points fund, and the six add up to fewer than 15), the first slot that
  breaks that rule, and unlearning a level, which also unlearns the skill's
  techniques that depend on it (the game's prerequisite table) and
  re-bases the points in 16 bits.
- Health: `save_health` writes yellow and red as shares of the maximum
  (red 0 unless positive), focus as its share, all in single precision and
  converted like the game's SSE instruction, the seed `0xFFFF` and the
  heal-pool and potion entries; `restore_health` loads them onto a body in
  double precision, including the original's quirk of taking the last
  saved entry's value for every tag, and leaves what no entry names.
- The player's, the duellist's and the corpses' records are the creator's
  defaults; a placed character carries its own record and gets its moveset
  from the first hand.

## Apparel

- `Outfit` lists garments by `Characters.rpk` model; `equip_outfits` fits
  each to the body parts, paints it from its appearance (or from the first
  object-database item worn as the model) and hides the body parts it
  covers. The fitting (the garment's weights from the nearest body vertex,
  its meshes) and the painting run on the async compute pool, and at most
  32 MiB of painted textures are made a frame; a body is dressed, 8 ms of
  dressing a frame, once everything it wears is in, before the animation
  systems bind the skinned parts in the same frame.
- Physique: a body root's `exa_assets::BodyMorph` (a placed character's
  record, `--physique` for the character view and the player) fits each
  garment as a variant morphed with the body (`ModelLoader::morphed_job`,
  built on the async pool and cached by the physique), weighted from the
  unmorphed body. See [`docs/formats/RFC.md`](../../docs/formats/RFC.md)
  (Morph sets).
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
  `HeldMoveset`: the original's moveset config (the weapon moveset and the
  Torch overlay, looked up under the race's own moveset), the stance class
  and the weapon class (the fists `0x80`). Dressing puts a wielded weapon
  in the main hand, the game crate puts a lit torch in its hand, and a
  system after dressing writes the config into the body's `Animator`
  (whose base stays the race's moveset) whenever the hands change. Only
  bodies with `Hands` follow it (the player and the duellist). See
  [`docs/PLAYER.md`](../../docs/PLAYER.md) (Moveset from held items).
- A wielded weapon's `Blade` carries its record as the original builds it
  (`exa_formats::weapon_record`): a plain model measured along its
  first-child chain of rigid parts, a factory weapon up to its assembled
  top, held by the character's own size, scale and encumbrance (its
  fighter's stats, `CharacterStats::holder`; an average human without a
  fighter) and its race (`RaceWeaponry`: the race index and `races.rdb`
  weapon fields and held offset, the human's by default). See
  [`docs/formats/OTHER.md`](../../docs/formats/OTHER.md) (Weapon record).
- A wielded weapon sits under the hand its grip names
  (`exa_combat::grip`, by its weapon class as the first held item: a
  polearm in the left hand), placed for the character's size and race
  (`RaceWeaponry::grip_holder`, `grip_bone`). See
  [`docs/PLAYER.md`](../../docs/PLAYER.md) (Held items and their grips).
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
