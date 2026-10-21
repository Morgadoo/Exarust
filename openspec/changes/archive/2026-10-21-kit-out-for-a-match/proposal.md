# Proposal

## Why

"Each rank has a corresponding loadout. Which loadout is used depends on
the tier of the match." The career prices loadouts and trims them by rank,
but which loadout a fighter takes into a match, the repair of a broken
list, the rank-up copy and an outfit's value are not in.

## What Changes

- The loadout a fighter takes: half its levels, at most the match's tier,
  or two under it for a Captain's later entrants; none for an ungeared
  match.
- The fighting skill the match copy is given (ten a level plus ten, under
  ten levels; else 100).
- The repair: a list not of six is rebuilt from the kit, all six trimmed,
  with the original's message.
- The rank-up copy of the loadout below.
- An outfit's value as shop lines.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: match loadouts.

## Impact

- `exa_game::hirelings::{match_kit, match_fighting_skill, repair_loadouts,
  promote_loadout, kit_value}`.
- Evidence: REA/Ghidra on `Exanima.exe` 0.9.5 (the match's loadout pass, the
  loadout window's repair, the rank-up copy, the outfit value, the line
  price). Notes: `context/re/arena-match.md`.
- Out of scope: applying a loadout to a body, the event weapon's draw, the
  loadout window.

## Phase

Phase 7, M7 "arena season": equipping the company.
