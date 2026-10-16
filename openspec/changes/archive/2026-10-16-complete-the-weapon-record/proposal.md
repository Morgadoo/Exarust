## Why

The original fights with a 28-element record per held weapon, built from
the item's weapon section, the weapon's length along its parts and the
character holding it, and rebuilt on every equip change. Exarust's
`WeaponRecord` kept only the damage fields (weight, balance, impact,
slash, crush, pierce, thrust, the striking part) and measured the length
from the mesh. The rest is what the coming combat changes run on: the
swing speed and shaping that time the attack clip (M22), the recoil that
sets cooldowns and stalls (L3, M22, M26), the stance blend, the arm
stiffness, the second hand's place on a haft (M25), the swing sound
factors, the AI's swing turn (L11), the reach and the weapon's mass as a
body (M24). The fists and claws fight through the same record, so fists
that strike (L3) need it too.

Phase 4 (Combat and items), milestone M4 "first duel"; master plan lane C,
change M21 `complete-the-weapon-record` (milestone MS1, original timing).

## What Changes

- A new `exa_formats::weapon_record` module, Bevy-free, holding the whole
  record as the original builds it: flags, type code, striking part
  (and its clamped start), weight and balance as stored and as held,
  impact, the four damage shares, swing speed, handling, recoil, swing
  shaping, stance blend, arm stiffness, length, sweet spot, reach, second
  hand, swing volume and pitch, AI swing turn, hit sound; the holding pass
  (armour encumbrance, character size, body scale, the weightless item,
  the half-sword hold) and the equip refresh (longest reach, the race's
  blend and swing-turn factors, the race's unarmed reach, the ghouls' arm
  stiffness, the Fael's half turn, the AI's thrust preference, the
  unarmed hands' mass).
- The weapon's length along its model's first-child chain of rigid parts
  (W21) and its sweet spot (W22): scene nodes now keep their box's half
  extents and whether they carry a collision body. An assembled factory
  weapon is one part up to its highest point.
- The built-in fist and claw sections (human fists, the ghouls' claws,
  the heavy claws of the large clawed races), chosen by race index for the
  right hand and by the race's clawed-hands word for the left, measured
  across the hand's box.
- The weapon mass table (by hands and size class, blended by weight) and
  its spread over a weapon's rigid parts.
- The shield record (item section `0x8000`, now parsed into object
  database items) and the factors it gives its holder (parry
  effectiveness, block leak, stamina cost, encumbrance share, with the
  Mobility and Endure techniques).
- A character's step reach (80·√scale, less with feral level and
  encumbrance).
- `exa_combat::damage::WeaponRecord` is the new record; held weapons are
  built through it in game (a plain model's chain, a factory weapon's
  top), and the duellist's reach is its weapon's record reach.
- Docs: `docs/formats/OTHER.md` (Weapon record), `docs/formats/RFC.md`
  (node box, collision body), `docs/PLAN.md` (Phase 4), crate READMEs.

Evidence:
- REA on `Exanima.exe` 0.9.5 and its disassembly: the record builder, the
  holding pass, the length and sweet-spot walks, the mass spread and its
  table, the equip refresh, the unarmed set-up with its three built-in
  sections and the hit-sound table, the shield record and its attach
  factors, the step reach (`context/re/combat-damage.md` §1,
  `master-combat.md` §3.2, §4 step 13, §10, §11.1, §14, `master-body.md`
  §6.2, §7.2; corrections appended there).
- Corpus: the 208 weapon items of the installed `objdb.rdb` (103 plain
  models measured in `Objects.rpk`, whose box tops equal their highest
  vertex within 0.05 cm; 98 factory items from their recipes; 7 whose
  model is not shipped), the hands of all 18 races' body models, the 22
  shield sections.

Out of scope:
- striking with the fists, the cooldowns and the energy gate (L3); the
  swing clock and the stall (M22); weapons and shields as bodies, where
  the mass would go (M24); the grip table and grip modes (L12, M25); the
  shield's attach, stamina and parries (M26, M27); the AI's attack choice
  and swing turn (L11); the step reach's consumers (C15, C16);
- each character's own size, encumbrance and feral level (M41, M42): until
  then every holder is a human of average height in no armour; the race
  fields are read from `races.rdb` here, and M40 may move them into the
  race record;
- the factory's striking extent of axe and hammer heads from assembled
  geometry (template bytes stay, as before);
- what sets the half-sword hold, and the readers of the clamped start and
  the handling (not found).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `combat`: the weapon record, the unarmed records and the shield record.

## Impact

- `exa_formats` (new `weapon_record`; `rfc::Node` box half extents and
  collision-body flag; `objdb::Item::shield`)
- `exa_combat` (`damage::WeaponRecord` and `damage::flags` re-exported
  from `exa_formats`)
- `exa_character` (held weapons built with the record)
- `exa_ai` (the duellist's reach from the record), `exa_world` (test item)
- `docs/formats/OTHER.md`, `docs/formats/RFC.md`, `docs/PLAN.md`,
  `crates/exa_formats/README.md`, `crates/exa_combat/README.md`,
  `crates/exa_character/README.md`
