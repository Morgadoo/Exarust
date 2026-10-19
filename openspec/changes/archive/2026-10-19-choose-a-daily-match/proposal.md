# Proposal

## Why

Each day the board fills six matches from seven templates. A template can be too early for that slot, already used up to its limit, or refused by a mask on the roll. The fixed tier-0 list does not make that choice.

## What Changes

- Seven gates: earliest slot, a limit, and a mask.
- Each try steps the roll by `× 0x08088405 + 1` and takes the high half of that word times 7.
- Refuse a limit above 0 that is already reached, a slot before the earliest, and a second step whose top three bits share a bit with the mask.
- Return the six template indices. The record copy and the fee scale stay out.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: which template fills each of the six daily matches.

## Impact

- `exa_game::career::choose_daily_matches`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the daily board filler. The seven gates were read as three integers each. No new corpus files.
- Out of scope: the clock that replaces the roll first, copying a template record, and scaling its fee.

## Phase

Phase 7, arena career. The daily choice, not the whole of `XA_Arena`.
