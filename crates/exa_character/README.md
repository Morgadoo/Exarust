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
- Weights and ranks: the equip pass's figure (the new item's per-bone
  armour weight less every worn item overlapping it on any bone, 0 when one
  of those has an equip lock over 50), the pack's worn weight (each worn
  item's template bulk byte), the arena's loadout rank (the four hands'
  weapon-or-shield rank and every worn item's body rank), and the prelude
  loader's import of an entry's item references into a live tier (the entry's
  items migrated from the live templates first, then the first hand pair, the worn list and the inventory, with an inventory repeat of a
  hand taking the hand's new id).
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
  covers. A garment gets one texture job and one stem: the first `&`
  material in node and part order,
  `exa_assets::model::painted_model_stem`, and every `&` part is painted
  from it, as the original rewrites all of them to the job's stem. The fitting (the garment's weights from its own seam groups, as
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
  the slot's select mask meets the visible mask; past 31 node-stack
  entries it logs the original's "Body part stack overflow" warning and
  walks on), the cull bits and the
  body mask (the union's innermost byte, which agrees with
  `Item::hidden_slots`). The dressed-mesh pass drives them: the running
  game plans each character from its items, so the mask decides which body
  slots merge (the per-node fallback still hides whole parts by the first
  byte). Real-data tests read `objdb.rdb` (`EXANIMA_DIR`) and check a helm
  on the head, boots on the feet and braies that hide nothing.
- `factory::PreviewJob` is the player's inventory preview job: it starts
  only when the shown item's model differs from the one the last job
  painted (the original compares model records; equal model names are the
  same record) at the side the setting gives (`preview_side`).
- `look` reads the race's look word (`races.rdb` `0x14`): the skin texture
  set (men's or women's; both sexes take the men's extra normal map), the
  six beards and their `bearda0N d/n` maps, the normal-map layers (weight
  `i` drives physique map `i + 1`, over 0.01 in double, none without a
  morph controller), the darkening (the record's feral level and age kept
  within 0…1 as the build's clamp leaves them) and the two random skin-map
  offsets
  drawn from the record's look seed (`search_seed`; a zero word is
  replaced). The hair rules read the worn set's head coverage union: bit 0
  shows the style's `?` roots, bit 1 grows no hair, and the look rebuild
  keeps, removes or rebuilds the hair by comparing the built and wanted
  hair codes. `limb_scale` is the per-limb leg scale's arithmetic (not
  called yet).
- `hair` builds a placed character's `CharExt` style under its head part
  (2 ms of styles a frame, so a crowd loads over several frames) by those
  head rules; `exa_assets::hair` gives the record's colour, specular, gloss
  and opacity. Hair is a separate model and is not part of the merged
  mesh. Not done: the `HarHPC1` texture set of profession bit
  `0x10000` (the variant's maps do not ship; `look::hair_textures` names
  both sets) and the creation scene's smoothing. Long hair hangs as cloth
  (`hair::hang_as_cloth`): each style node with the cloth marker (the two
  meshes of `hair hula01…05` and `hair husa02`) becomes an
  `exa_world::cloth` cloth pinned by its face flags, the style's roots
  with the hair's cloth record (`exa_physics::cloth_record::ClothRecord::HAIR`);
  on by default (`EXA_HAIR_CLOTH=0` keeps the styles rigid): the cloth
  stage's part index re-buckets only the bodies that changed and reads
  only the cells around its cloths, so exanima01's ~4 800 bodies no longer
  cost a full re-bucket every tick (`exa_world::cloth_parts`). A living
  character's hair never sleeps; each awake hair is offered only the body
  parts in the nine 6 m cells around it, as the original's grid sector
  walks them (offered every part of the level, exanima01 fell to 16 FPS).

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
  cache by its highest occupied layer over all twenty occupancy words
  (bones 1 to 20, so a piece that only occupies a bone past the eighteen
  slots still sorts by it), outermost last.
- `exa_assets::merge::merge` takes a slot's cull bits as the append's
  cull argument (`MergePart::cull`, tested against the part's
  per-triangle flags: `flags & cull != 0` drops the triangle, as the
  original's culling append does) and, for a part whose material has the occlusion mark
  (`MergePart::occlude`), fades the border of the marked region's vertex
  alpha. Each mesh keeps its own position-stream ids, moved past the
  meshes before it. The running game merges: `dress::assemble` builds the
  body tree from the built body and garment models, re-frames each garment
  on the body's slot frames and returns the plan; `equip_outfits` turns
  each step into `exa_assets::model::DressedPart`s and runs
  `model::merge_dressed`, which merges the built parts and spawns one
  sub-mesh per material run, bound to the character's skeleton with the
  root part as joint 0. The body nodes the merged mesh replaced stop
  drawing, and so do the body slots a garment covers (they are left out of
  the plan). Held items, hair and cloth stay as they are drawn today; no
  shipped material carries the occlusion mark, so that pass does nothing
  on shipped data (a tested rule). The built meshes keep their file
  triangle flags (`ModelNode::part_flags`), so a garment slot's cull bits
  reach the merge.
- A character without a `BodyModel` component (or with a garment model no
  object-database item is worn as) keeps the one-mesh-per-node draw. A
  real-data test merges Kade of `exanima01` into one mesh: 30 steps, 35
  parts, eight material runs and 5 624 vertices (`EXANIMA_DIR`; skipped
  when unset).

## The generator's runtime

The dressed-mesh plan, the slot chain and the skin job drive the running
game; the rebuild, spawn-height, background and saved-pose steps remain
tested rules of their own.

- `runtime` is the body tree the generator walks: each node's local and
  world frame, slot and layer, chunk kind, node and render flag words, the
  loader's owner word and its dynamics state. It reproduces the owner word
  write, the held marks (direct mesh children), the slot chain, slot
  numbering and re-framing on the base body's slot frames, the morph frame
  refresh, the dynamics build/split/rebuild passes, frame save (normalised
  with the reciprocal square root estimate) and rows-only restore, the
  state copy by slot, scene detach (the scene word written through the
  nodes that carry dynamics), cloth velocity, rest-point placement, the
  clone's cloth rule, the weight pass's walk along the slot tree, the
  merge-node collection the skin builder
  asks for (the chosen node, its marked parent at no offset, its marked
  mesh children at the saved slot frame or their own world position), the
  overlay marks and both saved-pose restores (a bone tree matched to the
  nearest child rest offset within 20 cm, and a flat list by slot; both
  re-derive only the local rows, as the world frame is what places the
  body). The rest vectors come from the engine's shape builder, which the
  caller supplies.
- `pose` puts a level's saved pose on a body model's scene tree: the bone
  tree first (records in depth-first order, each to the parent part's
  nearest child within 20 cm, an unmatched record skipping its subtree),
  the legacy flat matrices otherwise (slots 1 to 18 by the saved part
  table: pelvis, belly, chest, the right arm, the head, the left arm, the
  legs), and with neither the built pose. It gives every node its local
  frame and returns the character frame's translation (the restored root's
  world origin), so a corpse is spawned lying exactly where the level saved
  it instead of falling into place.
- `factory` is the start-up data: the eighteen slots' child tables, the
  skin-map side/format from the pixel-format setting (512 below 3, 1024
  from 3), the inventory preview's side (`preview_side`: 512 unless the
  high-quality flag and format 3 make it 1024), the item-database override
  and the material lock.
- `texture_job` describes the skin job the renderer runs: the race's colour,
  gloss and normal maps, the record's tone words, look seed and offsets,
  the darkening, the beard and its colour, the normal-layer blend, plus the
  first apparel texture the job carries. The running skin runs it:
  `SkinJob::composite_job` becomes an `exa_assets::SkinCompositeJob`,
  `ModelLoader::composited_skin` evaluates `chargen_d`/`_s`/`_n` and the
  beard overlays per texel (`exa_assets::skin_composite`) and uploads the
  sRGB colour, linear surface and linear normal maps, caching the material
  by the job's key (stem, tone, side, beard and layers). The
  maps are generated once per distinct look on the async compute pool
  (`PendingSkins`; the skin run draws the race maps until they are
  uploaded): 14 ms at 512² and 124 ms at 1024² on 8 threads, so a level's
  dressing stage takes no longer than the per-node draw's. The race maps are read unshifted (the
  seed's offsets move only the decomposition pass). The feral level is the job's
  decomposition amount: above zero, `composite_job` carries it with the
  seed's offsets and the composite draws the `chardecomp` passes; a body whose race texture set does not decode keeps the
  race maps plus tone.
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
  motion (the saved attack motions 5–12 written back as stance 3 and a zero
  phase, else the saved motion, else 0x13, or 0x19 for a profession word
  with `0x1000`),
  its lying flags (`0x1000`) and ground height (placement + standing −
  8 cm, or standing − 5 while lying; a saved pose places the body itself),
  the player's spawn (mode 1 stance 3 with its stance byte, mode 2 lying,
  0x13 otherwise, the frame's height + standing − 6 cm or the ground), and
  the corpse (stance 3, condition `0xA00`, the limp byte and down float
  cleared, a saved pose). It also gives the creator races their roles
  (`creator_role`: races 5–9 force role 11; 0x11 and 0x12 try 45 and 46)
  and the two material passes: run-time `&` materials clear their validity
  word and set `0x80000`, `*` placeholders clear their value and set `0x10`.
  `attach_plan` is the attach pass over the equip pass: the main hand's
  weapon, then one off-hand item (a shield from either hand first, else
  hand 1's torch, else its second weapon). Each weapon-section item's own
  `0x2000000` hand flag starts it on its flipped side (`item_flipped`; the
  main weapon's gives the dresser's `HeldFlipped`, a second weapon's its
  `OffHeld::flip`), and the last weapon's `0x1000000` sets the character's
  half-sword hold (`half_sword`, the `HalfSword` marker: its weapon records
  are built heavier and less balanced, without the two-hand factor).
  `offhand::insert_equipped` puts all of it on a spawned model, for the
  scene bodies (arena fighters) and a level's living placed characters,
  whose shields and second weapons are now drawn and held (27 shields and
  12 second weapons over the 13 shipped levels with characters; four in
  `exanima04`); the dead keep the moveset's hands but hang nothing, as the
  corpse spawn equips without attaching. The off-hand torch is not hung:
  the equip pass leaves it on its unflipped (unlit) side and Exarust's
  carried torch always burns; no shipped level places one. The appearance editor's hair refresh (`hair_rebuild`)
  always removes a body's hair and grows the style again only for a race
  with hair whose head word lacks bit 1; the scene attach and registration
  are the caller's. The viewer still stands living placed characters on the ground and does
  not set the motion, so those are plans; it places a level's dead from
  their saved pose: muscles off, every part on its saved world frame, the
  character frame at the restored root's origin, and the bodies registered
  at rest (one tick, then sleep). Only a `--corpse` without a saved pose
  keeps the built pose and the limp fall.

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
- At the same equip the weapon's model rigid nodes are turned into a part
  tree, each taking `1 / mobility` as its mass and its nearest rigid
  ancestor as its parent, and the original's two-pass redistribution
  (`exa_physics::weapon_mass::WeaponMass`) scales them with the record's
  table mass, the root's mass word then divided by the holder's size. The
  distributed parts stay on the weapon holder as a `WeaponMass` component.
  An assembled factory weapon has no stored rigid node and starts as the
  original's one merged part.
- A wielded weapon or torch becomes a solver body: every model node that
  stores a rigid body is a part, its parent the nearest rigid ancestor;
  the top part moves the item's root entity (which carries its blade,
  grip and carried fire), the others their own node entities. A
  `PendingProp` and an `exa_physics::grip::HeldGrip` carry its grip spec,
  holder body, hand and forearm entities, its held kind (weapon, shield or
  torch) and the parry-weld gate (off in the heavy-claws state and for a
  creature-body race, whose body rule flags carry the creature bit). The grip
  may be hung before the holder's body exists: `PendingGrip` then fills its
  holder key once the body's first part does. A model with no stored rigid
  node (an assembled
  factory weapon) stays a scene child of the hand: `assembled_item_body`
  fits one part with the original's factory-weapon shape builder, tested
  but not attached. The physics grip stage builds the links once both
  bodies exist and pulls the item to the hand; the item never meets its
  holder's body. The grip runtime (`grip_runtime`) reads the holder's clip
  every tick, applies the grip table entry's mode, steps an active flip
  (started by a `FlipWeaponRequest` for the character's entity, refused
  while the character switches or the weapon section has no flipped-side
  kind byte) and rebuilds its `Blade` record from the flipped weapon
  section, and a weapon whose own hand flag (`0x2000000`) says it starts
  flipped is placed half a turn about y and built from the flipped side
  at once. A collapsing holder lets its weapons and torch go
  (`GripRelease`: the grip stage frees them) while the shield stays on the
  arm; neither the collapse nor death changes the held item's welds (their
  limit-link writes are the legs', in `exa_physics::body`). When the switch clip passes 60% the runtime trades the two held
  weapons' blade slots, the attacker's primary and the `Hands` main and
  second weapon once, so `choose_held_movesets` picks the moveset again
  from the new main weapon (a two-hander and a one-hander trade "Two
  Handed" for the two-weapon moveset) (`condition::swap_due`,
  `swap_on_switch`).
- The off hand's item is attached by `offhand` after the dresser: a shield
  from either hand (its own model, body, a back pivot to its forearm and two face
  pivots to the left hand, and its soft weld, turned −5° about z on the hand) or a second
  weapon in the grip table's off-hand slot (entry 5, left hand) with its
  own `Blade` in the second slot. The spawn's attach plan decides which
  (`attach_plan`, `OffHeld`), and the shield's record travels with it
  (`Equipped::shield_record`) for the arm's block factors. The shield's
  equip writes the holder's block factors: the record's parry factor,
  leak and cost with its own Mobility and Endure techniques and a full
  arm (`exa_combat::damage::shield_guard`, kept on `exa_combat::block::ShieldHold`); the
  marker goes when the shield leaves or its grip is released
  (`drop_shield_holds`). The fighter's guard is not changed: the original
  blocks only a blow that strikes the shield's body, which
  `exa_combat::block::strike_held` finds in the pair stage. A second weapon turns with the character's starting
  flip (the main hand's flag), as every held item does.
- Each attached item carries what the equip refresh measures it by
  (weapon class, the record's start byte, the node's y extents and the
  race's reach): `HeldItemSpec`. A per-tick pass (`refresh_holds`) folds
  the first striker's into the character's `HeldRefresh`—the held weapon
  count (shields and torches have their own slots),
  the item's reach (start byte × extents in double plus 60 cm), its aim
  height, the aim point's weights on the item body's rest particles (cm)
  and the parry gate—as the original's equip and every swap do. The aim
  drive that reads the weights is outside this component, and the reach
  the AI reads is the equip pass's longest record reach, which overwrites
  the refresh's.
- The hands' contact mobility (how far a contact push moves a hand) is
  written by `body_words` on the ticks it changes: after every change of
  the hands an armed character's hands are at their base and an unarmed
  one's at half (a quarter in the heavy-claws state), as the original's
  emptying of the hands and its equip refresh leave them; every attack
  start in the heavy-claws state quarters both hands; a collapse empties
  an unarmed character's hands back to the base; and a gobbler's collapse
  gives each femur's limit link 10 cm of slack. The same refresh gives an
  unarmed character's hands the permanent weapon bit (`Ragdoll::set_fist`,
  the class word's bit 1, as the original's unarmed set-up marks the fists
  it holds) and takes it off when a weapon is in hand, so a fist striking
  a limb is silent on the impact path and meets a blade as a weapon in the
  classifier. A held shield's and a held torch's body take
  the held and mark bits (a weapon takes the weapon and mark bits), and a
  shield's link mobility equals its contact
  mobility (`exa_physics::grip::shield_words`).
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
  it; tied by the transpose of the part's turn, `spray::relative_frame`),
  against the swing (its axes made unit by the approximate reciprocal
  square root, `spray::jet_frame`), sending its droplets at 40 a second from one
  shared emission clock. The sprays tick in the fixed step after the body
  solver (droplets age, fall at a quarter of gravity, keep 0.95 of their
  speed a second; never touch anything) and are drawn each frame as
  camera-facing diamonds in one world-space mesh with
  `exa_render::spray::SprayMaterial`, between the last two ticks; the
  droplet texture `BldPrt02` loads with the first jet. `spray::DebugSprays`
  (`--debug-spray`) fires test jets from the player's or the character
  view's body every second. A world teardown (`LevelChangeRequest` with
  `teardown`) drops the jets (`BloodSprays::reset`); droplets in flight fly
  on. See [`docs/RENDERING.md`](../../docs/RENDERING.md) (Blood sprays).
