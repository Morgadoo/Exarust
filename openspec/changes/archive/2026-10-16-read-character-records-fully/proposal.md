## Why

Exarust reads a character record only up to its worn items. Everything the
original reads after them is lost: the feral level that weakens every
undead in the story levels, the saved health pools, the AI's combat skill,
the six skills with the technique chosen at each level (which give the
perks combat reads: Remise, Feint, Fend, Riposte, the shield perks,
Security), the inventory, the roles, the saved body state and the
thaumaturges' powers, and the looks before the worn list (physique, height,
age, voice pitch) that the spawn reads. The derived stats at spawn (M41),
encumbrance (M42), the swing clock (M22), clashes (M26), the damage rules
(M30), the physique morphs (M18) and saving (M47) all read them.

Phase 3 (Bodies), milestone M3 "standing man", and Phase 4's M4 "first
duel" (the perks). This is change M39 of the master plan
(`context/re/master-plan.md`, lane F, F2; body note B2 merged with combat's
read-combat-perks).

## What Changes

- `exa_formats::characters` reads a record whole, as the original's two
  readers do (version 7, and the legacy reader for versions 5 and 6):
  - the looks and voice it skipped: physique, height, age, voice pitch, and
    the voice number its name gives;
  - the tail: feral level, the saved yellow, red and focus pools (16-bit
    shares), combat skill, undead voice, four traits, experience, the six
    skills (points and the technique chosen at each level), the inventory,
    the roles (version 7 entries with their flags and variables; legacy
    lists as the old reader adds them, three old first roles replaced), the
    relations (counted; always none), the saved body state (heal pool,
    potion) and the powers sub-chunk;
  - every count checked, a record bounded by its own size (an actor's item
    database after it is not read), and an entry that is not a record an
    error.
- The perk set: one technique per 5000 points of a skill, at most six, and
  from the learnt techniques the sixteen perks the original's body reads
  (Close Combat Remise, Feint, Fend, Riposte; Armour Security; Shields
  Counter, Synchrony, Ward, Persist, Endure, Mobility; Insight Iteration;
  Concentration Sustain, Abstraction, Clarity, Volition), the highest
  Manoeuvring in Armour tier learnt and whether Deflection is learnt.
- `LevelCharacters::read_database` lists every database entry with its
  parse, so tools and tests see what is not a record.
- New `exa-tool characters <level|actors>` prints them.
- No game behaviour changes: nothing consumes the values yet. One test
  helper in `exa_world` builds its record with the new `Default`.

Evidence:
- REA and `exe.asm` on `Exanima.exe` 0.9.5: the version 7 reader and the
  legacy reader (field order, the three role replacements, the version 5
  and 6 relation shapes, the legacy powers entries), the inventory and
  relation-table readers, the technique-to-perk routine, the armour
  builder's Manoeuvring tier (the highest of techniques 1–3, × 0.3) and
  Deflection (technique 4), the save hook and the load (pools / 65535, the
  saved-state tags), the skill and technique name tables
  (`context/re/master-body.md` §7.1, §7.5, §11 and the new §17,
  `master-combat.md` §12, `roles-ai.md` §1).
- The installed game: every level's character database (478 version 7, 20
  version 6 and 4 version 5 records, 28 entries that are not records) and
  the 21 records of `actors.rpk`, pinned by corpus tests.

Out of scope:
- deriving anything from the values (M41: scale and standing height from
  height, Emax, reach and pins from the feral level, health from the
  pools; M42: encumbrance with the tier and Deflection);
- consuming the perks in combat (M22, M26, M27, M30) or the powers;
- the role database and parent roles of legacy records (AI, outside);
- saving records (M47); the original's last-entry quirk when it loads both
  saved entries is documented for M47, not applied by the parser;
- the appearance words (skin, hair, face styles), the AI's search seed, the
  last locale and the two record sub-chunks never seen in data
  (`0xCDEC00`, `0xCDEC12`), walked but not kept.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `character-stats`: records read whole (new requirement) and the perks
  their techniques give (new requirement).
- `asset-inspection-cli`: `exa-tool characters` (new requirement).

## Impact

- `exa_formats` (`characters.rs`: `CharacterRecord` fields, `Pools`,
  `Skill`, `Perk`, `Perks`, `Carried`, `Role`, `SavedState`, `Powers`,
  `LevelCharacters::read_database`, skill and technique names; tests in
  `characters_test.rs`)
- `exa_tool` (`characters` command)
- `exa_world` (one test helper)
- `docs/formats/RFC.md` (character records), `docs/PLAN.md` (Phase 3),
  `crates/exa_formats/README.md`, `crates/exa_tool/README.md`
