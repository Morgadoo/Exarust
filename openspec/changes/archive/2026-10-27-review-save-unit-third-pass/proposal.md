# Proposal

## Why

The third pass on the save unit added the scene presets the save unit
applies, the level transition's placement and follower rules, the scene
content lists, the scene character flush, the arena table writer and the
settings chunk hook on an exported level. A review against the
instructions found several rules ported from the pseudo-code that differ
from the original: the outdoor preset's draw arithmetic and rain stream,
the follower offset product, the re-attach controller word, the scene
content lists, the scene character flush order, the merchant chunk's
condition, the arsenal's list order and the settings chunk's tag.

## What Changes

- `exa_world::scene_env`: the outdoor and indoor presets as the
  instructions compute them (eased day draw, double sums, the engine's
  rain stream, the sun mode and sky setters).
- `exa_game::save_level`: follower offsets as row vectors times the spawn
  matrix; the re-attach controller stored as 1; scene content lists that
  can combine; the landing plan from the spawn matrix.
- `exa_game::arena_ids`: the scene character flush order with hand reuse.
- `exa_formats::arena_save`: the merchant chunk condition, shelf-major
  arsenal lists.
- `exa_formats::save_chunk`, `rfc_write`: the settings chunk's tag and
  write gate.

## Capabilities

### New Capabilities

### Modified Capabilities

- `save-games`: the presets, transition, flush and arena writer rules.

## Impact

- Crates: `exa_formats`, `exa_game`, `exa_world`.
- Docs: `docs/formats/SAVES.md`, the crate READMEs.
- Evidence: the save unit's instructions in `Exanima.exe` 0.9.5 (the
  presets, placement, migration, re-attach, transition, scene content,
  character flush, arena writer and reclassify routines, the settings
  writer, reader and load reset), the engine's vector helpers and the
  scene environment's rain, sun mode and sky setters.
- Out of scope: wiring the presets, the transition and the scene flush
  into the running game (no live arena scene load or level transition
  exists yet); the live arena tables; the fighter kit preparation; the
  player spawn and scene creation.

## Phase

Phase 7 — Game modes and content: saving and loading a story or an arena
career.
