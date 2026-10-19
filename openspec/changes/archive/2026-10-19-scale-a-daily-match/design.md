# Design

## Context

`choose_daily_matches` already picks six indices from a roll. The filler first stores a byte-reversed performance counter as that roll. After a template is copied, a scale writes the slot, the tier, a shown pair, an optional extra, and the two coin words.

## Goals / Non-Goals

**Goals:**

- `roll = swap_bytes(counter as u32)`. High bits of the counter are ignored.
- Then the existing six-template choice.
- Copy the template's kind, flags, tier, fee, and prize.
- Fee and prize unpack as copper + silver × 12 + gold × 144, multiply by `slot + 1`, and pack again. Gold is the low 16 bits of the count of 144.
- Bit 2 copies the slot into the tier. Otherwise `round_ties_even(slot × (1 − unit⁴))`.
- A tier below 0 becomes -1 before that.
- Kind 8 shows slot 0 and tier 5. Any other kind shows the slot and the tier.
- Bit 4 steps the roll and stores `high(roll × span) + 1`. Spans are 8, 11, 10, 14, 4, 4.

**Non-Goals:**

- Walking every typed field of the match record.
- The generator that produces the unit draw. The scale receives that unit.

## Decisions

1. The counter path calls `choose_daily_matches` after replacing the roll.
2. `place_daily_match` copies, then scales. The template is left as it was.
3. Rounding uses `round_ties_even`, matching the convert-to-integer with the default mode.

## Risks / Trade-offs

- `XA_Arena` moves from 30% to 32%. The game-modes section stays 0%. The headline stays 14.4% and 5.7%.

## Migration Plan

- Two functions beside the daily choice. `end_day` does not call them.

## Open Questions

- None for the counter rows and the scaled coins.
