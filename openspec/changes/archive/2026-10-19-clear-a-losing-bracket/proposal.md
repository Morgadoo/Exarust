# Proposal

## Why

An unwatched bracket still has to drop the losing records. The first group can be forced. Every later group is drawn, and the records that lose have their ids cleared.

## What Changes

- Score a team from the slots whose id is above 0: `(count × 0.5 + 0.5) × strength sum`.
- The first group keeps a forced id above 0 and does not step the roll.
- Any other group steps the roll by `× 0x08088405 + 1` and keeps the first team whose rounded weights pass the high half of the weight total times that word.
- Every other counted id becomes 0. More than one group reports that another group was resolved.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: clearing the losing records of a bracket.

## Impact

- `exa_game::career::clear_losing_bracket`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the bracket walk that zeros a losing record. The half and the hundred are the same floats as the unwatched draw. No new corpus files.
- Out of scope: spawning the rival teams, a physician's save, and menus.

## Phase

Phase 7, arena career. The bracket clear, not the whole of `XA_Arena`.
