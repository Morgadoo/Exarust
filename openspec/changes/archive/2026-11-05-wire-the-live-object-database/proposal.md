# Proposal

## Why

Wiring backlog system S2: the object database's three tiers existed in
`exa_formats::objdb` (instancing, promotion, migration, the remap tables)
with no game caller. The running game kept private copies instead: the
use cursor parsed its own database (its kit copies took local ids from an
empty local tier, colliding with the level's own records), the panels'
item records kept a cache of copies, the item catalogue copied and edited
cards by hand, a container popup's children were written into a decoded
copy, and the story save wrote no `#worldobjdb`, so a cleaned or repaired
item lost its condition at the next load.

Phase: Phase 4/6 item and save milestones of docs/PLAN.md (no PLAN.md edit
in this change).

Evidence (REA, Ghidra on `Exanima.exe`): the object database unit's
set-up and teardown (the three tiers' tags and libraries), its lazy base
loader, tier load, tier migration pass and id resolver; the level reader's
local-database chunk handler and its database-only entry; the save unit's
world object export, import, start check, save writer, load, pre-save
sync, player promotion, follower promotion, door transition and checkpoint;
the arena save's promotion pass; the arena round and arena start loaders'
scratch-database carry; the character-data unit's record promotion and
record copies; the new-game start. Corpus: every level of `Resource.rpk`
with a local database.

## What Changes

- `exa_assets::live_objects` (new): `LiveObjects`, the one live database
  (base tier read lazily, local tier filled by the level loader, world tier
  filled by a save's load), with a write log the decoded caches refresh
  from.
- `exa_formats::objdb`: the tier swaps (`load_local`, `import_world`,
  `clear_world`, `export_world`), the story's carried-items promotion and
  the arena's record promotion, a container's children written into its
  record; `rfc::Scene::local_database`, the level's records as a tier
  store.
- `exa_world`: the level load fills the live local tier;
  `LevelLocalRecords` is gone.
- `exa_game`: the use cursor's kits instantiate and write in the live
  database and the copy takes its template's place in the pack; the panels'
  records (`DollItems`) and the item catalogue read the tagged records from
  it; container popups write their children into it; the fighter and
  practice kit passes copy through it; saves run the world sync and export
  the world tier with its own library; loads import and migrate it; a level
  change promotes the player's local items first; a new story clears the
  world tier; `--kit-use` scripts kit releases.
- `exa_items::catalogue`: tagged records (local and world tiers) have
  kinds, children, usable bytes, burdens and shield records under their
  whole ids; a template's entries can be read again after a write.
- `exarust`: the loaded player's items resolve tagged ids in the save's
  world tier, untagged ones in the templates.

## Out of scope

- The arena round's scratch-database carry (Exarust promotes instead), the
  arena save's stash, arsenal, employee and scene-entity promotion, the
  followers' promotion.
- The saved level section (the local tier and the level's state in the
  save), so a load still reads the level from the archive.
- P5 use effects and P6 copied item bodies beyond the store they read.
