# Proposal

## Why

The fourth pass left the save unit's transition flow, world database
imports and placement reader as tested functions with no writer side and
no live caller. The save still could not export the two world databases or
the player's placement the original writes, and the arena load did not
regenerate the company's item lists after its id pass.

## What Changes

- `exa_game::slots`: `export_world_databases` and `WorldExport`, the
  world character database (flushed, saved) and the object world tier
  with its string library, each only when it holds a record.
- `exa_game::session`: `export_player_placement` (the first saved record,
  the respawn-pending flush rule) and `Session::placement`, written first
  after the header.
- `exa_game::arena_ids`: `regenerate_lists`, the employee and manager
  lists rebuilt from their loadouts, shelved with the saved-item flag, and
  every arsenal box deduplicated.
- `exa_game::level_transition`: `live_transition` and `LiveTransition`;
  the viewer runs the flow around every in-process level change.
- Docs: `docs/formats/SAVES.md`, the `exa_game` README.

## Capabilities

### New Capabilities

### Modified Capabilities

- `save-games`: world database export, placement export, arena list
  regeneration and the live transition flow.

## Impact

`exa_game` (slots, session, arena_ids, level_transition) and the
`exarust` viewer's level change. No save is written by the running game;
the writers produce the original's layout and only write through
`Slots::save_full` to a slot's own file.

## Phase

Phase 7 — Game modes and content: saving and loading a story or an arena
career (docs/PLAN.md).
