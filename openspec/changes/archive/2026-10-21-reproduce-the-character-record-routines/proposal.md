# Proposal

## Why

The character record was read only partly and none of the routines the
original runs on it existed: the writer, the constructor and the creator's
defaults, the item, hand and loadout rules, the equip conflict pass, the
skill-level rules and the save and load of health. Saving a game, the
arena's loadouts, trading and the skill screen all go through them.

## What Changes

- `exa_formats::characters`: every field of record versions 5–7 is read
  (identity and looks, hands and worn items with their flags, pools, seed,
  combat skill, traits, skills, inventory, roles with their weights,
  relations and per-level tables, saved state, loadouts, the reserved pair,
  powers with their blobs). Version 7 is written as the game writes it: the
  voice name from the game's number formatter, traits through the game's
  SSE conversion, names one byte per character, five techniques at most,
  and the powers, reserved pair and loadouts only when the game writes
  them. Every shipped version 7 record writes back byte for byte (two
  non-canonical voice names aside). The constructor (focus 0), the
  creator's reset (also in place), a copy through the writer, the record's
  own technique check and the pools loaded in double precision.
- `exa_formats::objdb`: the equip lock byte of an item's model section.
- `exa_character::record::CharacterData`: the live record with its
  reference count, release scope and staged list; the underwear rule; the
  equip conflict pass with its refusal; the inventory, hand, trade,
  loadout and remap rules; the skill levels and unlearning with the
  prerequisite cascade; the save and load of health; creator presets.
- Placed characters carry their record as `CharacterData` and take their
  second hand's shield or weapon into their moveset.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-stats`: the record is written back, defaulted, and its live
  rules run as the original's.

## Impact

- `exa_formats` (`characters`, `character_stats`, `objdb`, `encumbrance`),
  `exa_character` (`record`), `exa_world`, `exarust`, `exa_tool`
  (`characters` prints the new fields).
- Docs: `docs/formats/RFC.md` (character records: writing, defaults,
  saving, the live record), `docs/formats/OTHER.md` (the equip lock), the
  `exa_formats`, `exa_character` and `exa_tool` READMEs.
- Evidence: REA/Ghidra on `Exanima.exe` 0.9.5: both record readers, the
  writer and its number formatter and string copy, the constructor and the
  creator's reset, the role adder the legacy reader calls, the item, hand,
  loadout, trade and remap routines, the equip conflict pass, the skill
  routines and the prerequisite table in the executable's data, the save
  and load of health and the spawn's pool load. Instructions were read for
  every floating-point step (the save in single precision, the load and the
  spawn in double), the trait conversion, the trade's copy choice and worn
  loop and the unlearning cascade. Corpus: every record of the 29 levels
  and of `actors.rpk` parses, the version 7 ones write back, and the
  shipped object database's apparel drives the underwear, conflict and
  refusal rules.

## Out of scope

- The role database (`charroles.rdb`) and its parent roles, the role
  definition accessors and the role weights a legacy record would take
  from it.
- Copying item records between object databases and promoting a record's
  local items, the prelude character-database loader and the singleton
  streams and string guards of the engine.
- The item weight and loadout rank helpers, whose object-database fields
  are not identified.
- Wiring the save and load into a save package, the arena's loadout forms
  and the trade window; drawing the off hand's item.

## Phase

Phase 3, bodies (character records, M39 of the master plan), with the
item rules Phase 4 builds on.
