# Proposal

## Why

docs/PLAN.md, Phase 7 (game modes and content), the savegames3 paragraph:
the save unit's scene presets, the level transition's placement, the arena
table writer and the world clock's setter were tested functions with no
game caller. "Not wired yet: a live level export/import bridge, an arena
scene load calling the presets and the landing plan, and filling the arena
tables from the live arsenal, staff and tournament."

## What Changes

- A level load applies the scene preset its name selects
  (`exa_world::environment::level_preset`, `level_lighting_preset`):
  `arena1`–`arena7` and the story levels the indoor preset, `arenao1`–`5`
  the outdoor preset seeded from the clock, only `arenao1`–`4` allowed a
  night.
- The world clock's saved base: `exa_game::world_clock` (`restored`,
  `stamped`, `RestoreWorldClock`, `WorldClockPlugin`),
  `Session::world_clock` and `Session::stamp_world_clock`; a session start
  restores zero.
- The arena tables from live state (`exa_game::arena_tables::from_live`,
  `apply`, `Session::capture_arena`, `Session::apply_arena_tables`) and
  read back on an arena load.
- The level-change bridge as a tested plan
  (`exa_game::level_transition`) and a `PendingLanding` the play-level
  start honours.
- Review fix: a loaded arena save keeps its tables only when they write
  back byte for byte (`arena_save::read_tables_exact`), so a save the model
  cannot rebuild exactly keeps its chunks through a later save.

## Evidence

The arena's scene set-up routine, read from the assembly: an arena index
below 7 takes the 2.0 indoor variant, otherwise the outdoor preset with the
night flag `table[index] AND caller flag` and the caller's seed; its only
caller passes the flag 1 and the engine's tick count. The executable's
level-name table (sixteen-byte entries) lists `arena1`–`arena7` then
`arenao1`–`arenao5`, and the night table holds 1 for indices 7 to 10 and 0
for 11. The story levels' environment callback picks the 1.8, 1.4 or 2.0
indoor variant per level. The player-data writer stamps the clock getter's
seconds into the header; both readers call the clock setter.

## Out of scope

A live level import and export (Exarust's loader keeps no export scene and
a level change restarts the process), a live save/load path in the running
game, the day record track (the night, rain and cloud dimming are not
drawn), the arena id work over the live object database, and the arena
chunks' fields with no live model (kept from the previous save).
