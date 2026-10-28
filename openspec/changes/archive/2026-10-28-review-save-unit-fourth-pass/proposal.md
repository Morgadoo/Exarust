# Proposal

## Why

The fourth pass on the save unit added the three level-load entry points
and the import order, the fighters' two kit passes, the import of a save's
world databases, the old arena save's record repair, the level transition's
ordered flow, the outdoor preset's view value and a fix to the migration
list. A review against the instructions found rules that differ from the
original: the record repair walked lists the routine never touches, the
transition disabled saves on a failed save-section import and skipped the
follower re-attach pass without followers, and several scene steps were
misnamed or missing (the sun-mode setter read as a sky mode, the named
load's environment word read as the sun mode, a view reset that is the GI
manager's, a terrain probe that depended on parsing, the menu load's form
close and current-scene store, an invented item GUI reset).

## What Changes

- `exa_formats::arena_save`: the record repair compacts the player's worn
  list, each loadout's worn list and the carried items, then re-derives.
- `exa_game::level_transition`: the save-section path always lands; only
  an archive import failure disables saving; the re-attach pass always
  runs; the portal word takes the previous level id.
- `exa_game::save_scene`, `save_kit`: the corrected scene, import, menu
  and combatant steps.
- Docs: `docs/formats/SAVES.md`, the crate READMEs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `save-games`: the level import, the transition flow, the kit passes,
  the record repair, the world database import, the preset view value and
  the flush's attached nodes.

## Impact

- Crates: `exa_formats`, `exa_game`, `exa_world`.
- Evidence: the save unit's instructions in `Exanima.exe` 0.9.5 (the
  outdoor preset, the scene character flush, the record repair and its
  two compactions, the kit preparation and equip passes, the combatant
  scene, the three level loads, the import and its terrain probe, the game
  and menu scene set-up, the transition) and the scene environment's
  sun-mode setter.
- Out of scope: wiring the scene loads, the import, the transition, the
  kit passes and the record repair into the running game (the level
  loader keeps `exa_world`'s scene model and the load does not re-encode
  the world character database); the engine, renderer, audio and GUI
  calls at the named steps.

## Phase

Phase 7 — Game modes and content: saving and loading a story or an arena
career.
