# Proposal

## Why

Phase 2 of `docs/PLAN.md` (milestone M2, "walkable world") needs the original
levels inside the Bevy runtime. Static meshes and materials already decode.
Levels add two things on top: placed props (thousands per story level) and a
tile grid that selects wall and floor pieces from modular sets. This change
brings levels and props into Bevy so they can be looked at. The grid builder
follows in its own change.

## What Changes

- Decode the level sections of chunk trees: tile-grid header (wall and floor
  set names, dimensions, the three per-cell planes) and placed props (object
  name, placement transform, parent).
- Convert decoded meshes and materials into Bevy meshes and materials, with
  textures from the installation.
- `exarust model <pack> <entry>` shows one model; `exarust level <name>` shows
  a level's own scene nodes plus every prop placed from the object library;
  both with a free-fly camera.
- `exa-tool corpus levels` reports level-section decoding over all levels.

Evidence: the level and prop readers in `Exanima.exe` (examined with REA),
corpus statistics over all level files, and screenshots of the Bevy view.

Out of scope:
- generating walls and floors from the tile grid (next change);
- terrain (`.rft`);
- lights' exact parameters (lights are approximated);
- characters (`CAC00001`) and the local database (`DBC0`);
- collision and gameplay.

## Capabilities

### New Capabilities
- `level-decoding`: reading the tile-grid header, per-cell planes and placed props of a level file.
- `world-viewer`: showing original models and levels in the Bevy runtime with a free camera.

### Modified Capabilities

## Impact

- `crates/exa_formats`: new `level` module; `Scene` gains grid and props.
- `crates/exa_assets`: mesh/material/texture conversion and caching.
- `crates/exa_world`: level spawning.
- `crates/exarust`: `model` and `level` commands, fly camera.
- `crates/exa_tool`: `corpus levels`.
- `docs/formats/RFC.md`, `docs/PLAN.md`.
