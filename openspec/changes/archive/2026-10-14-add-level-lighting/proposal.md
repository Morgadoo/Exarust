# Proposal

## Why

Outdoor levels had only the viewer's flat ambient light, and every level shared
one ambient colour. Each level file has an environment record, and the game
applies per-level lighting presets (Phase 2/5 of `docs/PLAN.md`).

## What Changes

- `exa_formats::environment`: the `0xEC00DC00` record (ambient colours, vertical
  extent, sun settings), exposed on `Scene`.
- `exa_world::environment`: the original's presets. Indoor levels get no sun,
  the outdoor arenas get a shadowed sun and fog, and "auto" lights a sun only in
  levels without lights. The ambient colour comes from the record.
  `spawn_level` applies the lighting.
- `exa-tool corpus levels` counts environment records.

Out of scope: the sky with its time-of-day colour tables, weather (rain,
clouds, wetness), the secondary ambient, the reverb term.

## Capabilities

### New Capabilities
- `level-lighting`: lighting levels like the original.

### Modified Capabilities

## Impact

`crates/exa_formats`, `crates/exa_world`, `crates/exa_tool`, docs.
