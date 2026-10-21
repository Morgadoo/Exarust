# Proposal

## Why

The arena's employees, hireables, merchant stock and tournament lineups had
no counterpart beyond the career rules: nothing saved or restored their item
ids, nothing generated a hireable's or opponent's outfit, the manager form's
roster and loadout rules were absent, and the arsenal icon sheet was not laid
out. A first pass written against the decompilation carried several
rule errors (draw order, id remaps, float precision, an invented icon table),
which this change corrects while adding the rest.

## What Changes

- `exa_formats::arena_save`: the company, day, tournament, employee,
  hireable and merchant chunks.
- `exa_game::arena_ids`: the save-id preparation order, the two post-load
  remaps (employee reference and character reference) and the fighter rank
  (six skills' levels summed, halved, capped at five).
- `exa_game::arena_outfit` and `arena_outfit_data`: the profession grids, the
  three weighted picks, the item-row fillers, the by-level and random NPC
  generators and the profession level grants, all on the engine's
  linear-congruential generator in the original's draw order, with the
  double-precision sums the original narrows once.
- `exa_game::arena_spawn`: the spawn plans for beasts, hires and floor
  probes, as data (no body spawner calls them yet).
- `exa_game::manage`, `loadout` and `manage_icons`: the roster cell text and
  ink, the health fill, the daily upkeep, the loadout form's bind, open,
  reload and refresh rules, the hire fee, and the icon atlas growth.
- `exa_game::arena` and `tournament`: restoring a saved special match,
  the weapon-row check, the prize copy's state bytes, and the fallback that
  settles the first undecided tournament event.
- `exa_game::staff`: fighter numbering and the profession-word check.

## Capabilities

### New Capabilities

- `arena-staff-outfits`: employee records, outfits and the manager form's
  rules.

### Modified Capabilities

## Impact

- New: `exa_formats::arena_save`, `exa_game::{arena_ids, arena_outfit,
  arena_outfit_data, arena_spawn, manage_icons}`.
- Changed: `exa_game::{arena, arena_hub, arena_match, career, loadout,
  manage, staff, tournament}`, `docs/UI.md`, `docs/formats/SAVES.md`, the
  crate READMEs.
- Evidence: REA (Ghidra 12.1.4) on the arena unit, the character-data helpers
  and the object database accessors it calls, with the assembly read where
  the decompiler hides double-precision sums, signed compares and the
  rounding conversions. The tables are the shipped arena data; no retail data
  is embedded in tests.
- Out of scope: a body spawner that uses the spawn plans, the object
  database's item-fact pass, the form windows themselves, the stop-ambience
  flag of the hub (a guess), the icon handle each draw callback picks, the
  final armour mode in the maximum-rank path, and the writer's chunk order in
  `save_package` (unchanged).

## Phase

Phase 7, game modes and content: the arena career's staff, outfits and
manager forms.
