# Proposal

## Why

The daily choice still starts from a roll the caller already holds. The image replaces that roll from a performance counter, copies the chosen template, and scales its fee and prize by one more than the slot.

## What Changes

- Reverse the bytes of the counter's low 32 bits and use that as the roll, then choose the six templates.
- Copy a template onto the day's match.
- Multiply the fee and the prize by one more than the slot.
- Bit 2 keeps the slot as the tier. Otherwise the tier is the slot times one minus the unit's fourth power, rounded nearest even. A negative tier becomes -1. Kind 8 shows 0 and 5. Bit 4 draws one more than the high half of the stepped roll times the slot's span.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the counter that replaces the roll, and the copy and fee scale of a daily match.

## Impact

- `exa_game::career::choose_daily_matches_from_counter` and `place_daily_match`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the counter byte reversal, the coin multiply, and the tier curve. No new corpus files.
- Out of scope: the typed field walker, and the separate generator that supplies the unit draw.

## Phase

Phase 7, arena career. The counter and the scale, not the whole of `XA_Arena`.
