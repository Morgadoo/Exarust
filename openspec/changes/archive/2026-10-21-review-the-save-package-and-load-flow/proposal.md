# Proposal

## Why

The second pass of the save lane reproduced the package container (the
placement and delete rules, the untouched bytes a rewrite keeps), the
reader's defaults and load flow and the writer's chunk order. Checked
against the disassembly, one writer rule was wrong: the arena writer put
the employees before the company record, following the chunk ids, but it
writes the company and the extra record first. The rules were also not
written into the specs, and the container could only be checked against
real saves by a test that needs a copy of them.

## What Changes

- The arena writer emits the journal, the arsenal, the stash, the shop
  stock, the company record, the extra record, the employees, the
  merchant, the hireables and the tournament, in that order.
- The story writer order (discovery, journal, global experience, memory
  flags, transition) and the save section order (world characters, player
  data, world objects, the level) are specified.
- `exa-tool saves <file>` reads a save package without writing anything
  and reports whether the package and its player data rebuild unchanged.
- The duplicate-id pass of the arena compaction is documented as scanning
  from the end of the list.

## Capabilities

### Modified Capabilities

- `save-games`: the writer orders.

## Impact

- `exa_formats::save_package` (`ARENA_CHUNK_ORDER`), `exa_formats::arena_save`
  (a doc comment), `exa_tool` (the `saves` command), `docs/formats/SAVES.md`,
  the `exa_tool` README.
- Evidence: REA/Ghidra disassembly of the original's save writer, the
  package put and delete routines, the player-data reader and the arena
  reader of `Exanima.exe` (the savegames evidence pack).
- Out of scope: wiring saves into the running game (the game holds no
  session, company or story state to save), the old-format merchant and
  tournament reads, the arena id pass, item classification and the
  regeneration of hireables (they need the object database), the level
  and world database imports and the scene writer.

## Phase

Phase 7, game modes and content: saves.
