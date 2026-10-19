# Proposal

## Why

A match nobody is watching still names a winner. Each counted team is scored from its size and its strength, then a roll picks among the squared weights. That draw is not in the career yet.

## What Changes

- Score a counted team as `(count × 0.5 + 0.5) × strength sum`.
- Leave out a fighter set aside, and in mode 16 leave out the player's team.
- Step the roll by `× 0x08088405 + 1` and pick the first team whose rounded weights reach the high half of that word times one more than the total.
- An empty counted list steps the roll and names no team.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the unwatched draw of a winning team.

## Impact

- `exa_game::career::draw_unwatched`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the unwatched team draw. The half, the hundred, and a zero starting maximum were read as floats. No new corpus files.
- Out of scope: the bracket that clears losing records and can force the first group's winner, a physician's save, and menus.

## Phase

Phase 7, arena career. One draw, not the whole of `XA_Arena`.
