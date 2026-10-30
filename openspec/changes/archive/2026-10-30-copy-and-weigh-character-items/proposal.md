# Proposal

## Why

Third pass over the character data unit (docs/PLAN.md Phase 4, combat
and items: equipment, weights and the item records a character carries;
the prelude's character entries feed Phase 7). The unit copies item
records between object database tiers, imports a character entry's own
items into the live database, weighs armour for the equip pass and ranks
loadouts for the arena; Exarust had none of these.

Evidence: the original's assembly for both record copies, the two entry
readers, the item import with its whole-tier migration, the equip weight
and its armour weight callee (with the bone weight table and the two
double constants read from the executable), the template bulk sum, both
loadout ranks and the eleven value block accessors (REA and the objdump
listing), the pseudo-code used only for structure. The review found the
whole-entry builder's eight-byte step that never skips a body, the
import's separate chunk walk, the migration the import runs first, a
weapon section deciding a hand's rank even when short, and the stride
accessor's single argument; each is corrected.

## What Changes

- `exa_formats::objdb::ObjectDb::copy_record`, `copy_record_from` and
  `copy_instance`: the two record copies with their string rules.
- `ObjectDb::migrate_store`: the template migration over a store held
  outside the database.
- `exa_formats::characters::read_entry` (the whole-entry builder's walk),
  `read_entry_items` (the import's walk), `LevelCharacters::read_entries`
  and `read_character_entries`.
- `exa_character::record::CharacterData::import_items` (migrates first),
  `equip_weight`, `worn_weight`, `loadout_rank`; `Item::armour_weight`,
  `Item::loadout_rank`, `Item::worn_rank`.
- `exa_formats::roles::ValueBlock` and `set_value_word`.

## Out of scope

- Wiring: no running caller exists yet for the prelude import, the equip
  pass's weight, the arena's loadout rank or the save's instance copy;
  they stay tested functions with the original's numbers.
- The spawn builder's pool writes onto a placed character's health and the
  creator's name-based lookup (the placed spawn carries no health yet).
- The meaning of the value block's areas and counts.
