# Proposal

## Why

Wiring backlog G5 and the S2 follow-ups. Round 51 gave the running game
its live object database, but a save still wrote no level section: a load
read the level from the archive, so a dropped, moved or repaired level
object was lost, and the local tier the save should carry was never
written. The arena save promoted only the manager, the next practice
fight promoted the player's kit instead of carrying it, and the `.rdb`
reader read each table's last slot as a record (every level's own
database carried one bogus entry; `exanima01`'s read as id `25624963`).

Phase: Phase 4/6 save and item milestones of docs/PLAN.md (no PLAN.md edit
in this change).

Evidence (REA, Ghidra on `Exanima.exe`, checked against the assembly): the
save unit's save routine (section order, the arena promotion call, the
exporter's call and the locale name), the exporter's scene write (the
local-database chunk is the live local tier), the save unit's full load,
scene load and transition (the importer's hook: the tier migration for a
save's section, the archive pass for the archive's file), the level
reader's local-database chunk (when the hook runs), the archive pass
(migration first, then the container fill), the arena save's promotion
pass and its list and scene-entity helpers, the travelling followers'
promotion, the fighter-kit preparation (its hand-pair table, read in the
instructions) and the practice next-fight loader, and the database
loaders of the object database, the character databases and the string
library. Corpus: every `.rdb` of `Resource.rpk` and every level's own
database (the last table slot of each); `exanima01` and `arenahub` sections
written and read back.

## What Changes

- `exa_formats::rdb`: `table_len / 16 − 1` records; the slot after the
  last entry is not read.
- `exa_assets::live_objects`: a level load migrates the whole local tier.
- `exa_world`: `LevelSource` (the section a level was read from and the
  placed objects it spawned), `PlacedIndex` on each spawned placed object,
  `SavedLevelSection` and `LevelLoad::begin_from` (a save's own section
  instead of the archive's file).
- `exa_game::level_save` (new): the save's level section from the loaded
  section with the live local tier and the live placed objects written
  over it; every save writes it after the world databases; a story load
  hands the save's section to the level load.
- `exa_game::save_live`: the arena save's promotion pass (manager, stash,
  arsenal boxes, employees' records and lists, scene entity ids); the
  travelling followers' promotion; the level change does not promote while
  the kit's temporary database carries the player's kit.
- `exa_game::save_kit`, `flow`: the next practice fight prepares the kit
  before the arena's load and equips it into the new local tier after it.
- `exa_game::container_fill`: the fill writes the level's records and does
  not run on a level read from a save.
- `exarust`: `--hub-stash`; `--gui-drop` drops a pack entry it already holds.

## Out of scope

- The props', doors' and levers' live positions, the scene characters and
  the settings chunk's live words in the section (the loaded section's are
  written).
- An arena slot's load reading the hub from the save's section (the hub is
  loaded before the slot is read).
- Live travelling followers (none travel yet; the promotion is ported).
- The arena hub-to-fight scratch carry for company fighters other than the
  player.
