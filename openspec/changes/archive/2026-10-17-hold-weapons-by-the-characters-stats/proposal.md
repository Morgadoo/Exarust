## Why

The weapon record (M21) holds every weapon and fist by an average human
in no armour (size 1, scale 1, encumbrance 0), and keeps its own step
reach beside the character stats' (M41): two lanes landed the same rule
twice, one in single precision and one in double. Now that every
character spawns with its size, scale (M41) and encumbrance (M42), the
original's holding pass and unarmed set-up read them, and its init
computes the step reach once, in double precision.

Phase 4 (Combat), milestone M4 "first duel": the merge of lane F (M41,
M42) with the combat lane's weapon record (M21), as both reviews asked
(`context/re/master-body.md` §18 and §19 merge notes).

## What Changes

- `CharacterStats::holder`: the character's size, scale and encumbrance
  as the weapon record's `Holder` (no weightless item yet).
- Dressing (`exa_character`) holds a wielded weapon's record and the
  fists' records by the fighter's own holder instead of the average one.
- One step reach: `weapon_record::step_reach` computes it in double
  precision as the original's init does (M41's arithmetic);
  `CharacterStats::step_reach` returns it in metres.

Evidence:
- REA and `exe.asm` on `Exanima.exe` 0.9.5, as recorded by M21, M41 and
  M42: the holding pass reads the owner's encumbrance as its grip and the
  size as the reach and unarmed stiffness factor, the unarmed set-up the
  same encumbrance, and the init's reach in double
  (`context/re/master-body.md` §18, §19; `context/re/combat-damage.md`).

Out of scope:
- the weightless share (the shield-lightening gauntlets are unpowered in
  every shipped record; the armour pass's flag is not kept yet);
- re-holding on equipment changes (equipment does not change yet);
- the half-sword hold (what sets it was not found).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `combat`: a held weapon's and the fists' records are held by the
  character's own size and encumbrance (modified Weapon record, Unarmed
  records).

## Impact

- `exa_formats` (`character_stats.rs`: `holder`, `step_reach`;
  `weapon_record.rs`: `step_reach` in double; tests)
- `exa_character` (`held_record`, `fist_records`, `equip_outfits`; tests)
- `docs/formats/OTHER.md`, `docs/formats/RFC.md`, `docs/formats/RSQ.md`,
  `docs/PLAN.md`, `crates/exa_character/README.md`
