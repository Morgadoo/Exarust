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

## Creation records

A creator record is this crate's `CharacterRecord`: its six `Skill`s
(`points` and the technique chosen at each level) and its four hand slots,
worn list and carried list are the fields the creator edits. `exa_game::create`
owns the starting skills, the mode's gear filter, the experience budget and
the page flow, and writes the chosen name, hands, worn and carried lists,
skills and full pools into the spawned record; this crate reads the stored
record. The creator's two
physique values and the body morphs they select remain the body's concern
(see [`docs/formats/RFC.md`](../../docs/formats/RFC.md), Morph sets).

## Apparel

- `Outfit` lists garments by `Characters.rpk` model; `equip_outfits` fits
  each to the body parts, paints it from its appearance (or from the first
  object-database item worn as the model) and hides the body parts it
  covers. The fitting (the garment's weights from its own seam groups, as
  the body's, its meshes) and the painting run on the async compute pool, and at most
  32 MiB of painted textures are made a frame; a body is dressed, 8 ms of
  dressing a frame, once everything it wears is in, before the animation
  systems bind the skinned parts in the same frame.
- Physique: a body root's `exa_assets::BodyMorph` (a placed character's
  record, `--physique` for the character view and the player) fits each
  garment as a variant morphed with the body (`ModelLoader::morphed_job`,
  built on the async pool and cached by the physique), weighted from the
  unmorphed body. See [`docs/formats/RFC.md`](../../docs/formats/RFC.md)
  (Morph sets).
- Skin: a body's `exa_assets::SkinLook` (a placed character's record tone
  and sex, set by the spawner) re-binds its race skin parts to the built-in
  skin material of that tone, made once per race, tone and sex
  (`apply_skin_looks`). Bodies without a look keep the middle tone.
- Underwear: humans marked `WearsUnderwear` (the player, the character
  view's human body) wear the object database's braies (item 8) beneath
  their outfit whenever nothing in it covers the groin on the innermost
  layer (`needs_underwear`), as the original keeps its human records.
  Placed characters are dressed exactly as stored. See
  [`docs/formats/OTHER.md`](../../docs/formats/OTHER.md) (Underwear).

## Layers, looks and hair

- `layers` models the builder's eighteen body slots (pelvis, belly, chest,
  head, the two arm and leg chains and the hands, each with its parent and
  children), the four-digit node names that place a garment part in one
  (`0906Chest`: chest, sub-part bits `0b110`), and the builder's per-garment
  passes: the coverage pass (the garment's occupancy bytes against the worn
  set's union of the item body's `0xA8` words, giving the cover its node
  bits are scored against and the garment's visible mask), the slot
  assembler (best groups per slot, child slots pruned under the group's
  parent-slot node, the same-bits fallback, the first group chosen where
  the slot's select mask meets the visible mask), the cull bits and the
  body mask (the union's innermost byte). These are tested rules; the
  running game still hides whole body-part meshes by the first byte
  (`Item::hidden_slots`), which the body mask agrees with. Real-data tests
  read `objdb.rdb` (`EXANIMA_DIR`) and check a helm on the head, boots on
  the feet and braies that hide nothing.
- `look` reads the race's look word (`races.rdb` `0x14`): the skin texture
  set (men's or women's; both sexes take the men's extra normal map), the
  six beards and their `bearda0N d/n` maps, the normal-map layers (weight
  `i` drives physique map `i + 1`, over 0.01 in double, none without a
  morph controller), the darkening and the two random skin-map offsets
  drawn from the record's look seed (`search_seed`; a zero word is
  replaced). The hair rules read the worn set's head coverage union: bit 0
  shows the style's `?` roots, bit 1 grows no hair, and the look rebuild
  keeps, removes or rebuilds the hair by comparing the built and wanted
  hair codes. `limb_scale` is the per-limb leg scale's arithmetic (not
  called yet).
- `hair` builds a placed character's `CharExt` style under its head part
  (2 ms of styles a frame, so a crowd loads over several frames) by those
  head rules; `exa_assets::hair` gives the record's colour, specular, gloss
  and opacity. Not done: the `HarHPC1` texture set of profession bit
  `0x10000` (the variant's maps do not ship; `look::hair_textures` names
  both sets), hair as cloth and its smoothing.

## The dressed mesh

- `dress` is the merge list the original's skin builder appends: each
  garment's chosen nodes in slot order (1 to 18), gated by the slot's select mask against the
  garment's visible mask and carrying the cull bits of
  `layers::cull_bits`; then the body's visible slots in slot order, with no cull bits;
  then the overlays (the meshes numbered past slot 18, marked
  `(flags & !0x30000) | 0xC0000`). `DressedPlan` also carries the body's
  visible mask and slots and the per-slot impact material: the garment that
  leaves the most of the slot's uncovered coverage word, compared as the
  original's 64-bit word, gives the body part its hit-sound class.
- Garments are planned in their layer-level order whatever order the worn
  list gives them: the original inserts each into the character's apparel
  cache by its highest occupied layer, outermost last.
- `exa_assets::merge::merge_parts` carries the cull bits on the merged
  triangles and, for a part whose material has the occlusion mark, fades
  the border of the marked region's vertex alpha. Each part keeps its own
  position-stream ids. The running game still draws one mesh per node, so
  no caller merges yet; the plan, the merge and its tests are the source of
  truth for the single mesh.

## The generator's runtime

These are tested rules the running game does not drive yet.

- `runtime` is the body tree the generator walks: each node's local and
  world frame, slot and layer, chunk kind, node and render flag words, the
  loader's owner word and its dynamics state. It reproduces the owner word
  write, the held marks (direct mesh children), the slot chain, slot
  numbering and re-framing on the base body's slot frames, the morph frame
  refresh, the dynamics build/split/rebuild passes, frame save (normalised
  with the reciprocal square root estimate) and rows-only restore, the
  state copy by slot, scene detach (the scene word written through the
  nodes that carry dynamics), cloth velocity, rest-point placement, the clone's cloth rule, the merge
  order along the slot tree, the merge-node collection the skin builder
  asks for (the chosen node, its marked parent at no offset, its marked
  mesh children at the saved slot frame or their own world position), the
  overlay marks and both saved-pose restores (a bone tree matched by
  position, and a flat list). The rest vectors come from the engine's shape
  builder, which the caller supplies.
- `factory` is the start-up data: the eighteen slots' child tables, the
  skin-map side/format from the pixel-format setting (512 below 3, 1024
  from 3), the item-database override and the material lock.
- `texture_job` describes the skin job the renderer runs: the race's colour,
  gloss and normal maps, the record's tone words, look seed and offsets,
  the darkening, the beard and its colour, the normal-layer blend, plus the
  first apparel texture the job carries. `exa_assets::human_skin` draws the
  parts it can; the tone colour, the beard and the blend are not drawn.
- `rebuild` keeps the look rebuild's own steps (job reset and first
  apparel, frame save, dynamics rebuild, frame restore, the root lift by
  the new standing height, the double-precision body scale), the
  dressed-mesh plan (per garment, in layer order: chosen slots, visible
  mask, merge order, plus the per-slot impact material; the body's mask and
  merged slots), a texture-only rebuild and the creation scene's hair roots. `background` hands a runner the
  rebuild's build and apply steps apart, or runs both inline.
- The dressing and texture-only passes' feral overlay: the underwear (item 8) of a feral
  character takes damage and dirt `round_to_even((2 − f)·f·255)` and the
  record's look seed; every other first apparel keeps its stored appearance
  and its own seed (`retexture_layer`). `preview` and `retexture_only` are
  the two public wrappers.
- `spawn` plans the spawns the levels and arena call: a placed character's
  motion (the saved attack motions 5–12 written back as stance 3, else the
  saved motion, else 0x13, or 0x19 for a profession word with `0x1000`),
  its lying flags (`0x1000`) and ground height (placement + standing −
  8 cm, or standing − 5 while lying; a saved pose places the body itself),
  the player's spawn (mode 1 stance 3 with its stance byte, mode 2 lying,
  0x13 otherwise, the frame's height + standing − 6 cm or the ground), and
  the corpse (stance 3, condition `0xA00`, the limp byte and down float
  cleared, a saved pose). It also gives the creator races their roles
  (`creator_role`: races 5–9 force role 11; 0x11 and 0x12 try 45 and 46)
  and the two material passes: run-time `&` materials clear their validity
  word and set `0x80000`, `*` placeholders clear their value and set `0x10`.
  The viewer still stands placed characters on the ground and does not set
  the motion, so these are plans.

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

The sprays also carry the original's one scene node description
(`spray::BloodSprays::scene_node`: the `_BloodSpray` node, the built-in
`_BloodPrt02` material over `BldPrt02`, and the node and material flag
words). Nothing builds that node yet.

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
