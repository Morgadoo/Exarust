# Design

## Context

Hiring already stops at seven combatants and one of each specialist, and it does not touch the copper. Recruitment compares the purse with 12 copper before anyone joins.

## Goals / Non-Goals

**Goals:**

- Refuse a hire below 12 copper without changing the roster or the spent total.
- Refuse a full roster or a second specialist without spending.
- Subtract 12 and record 12 spent when the hire joins.

**Non-Goals:**

- The price shown on a candidate before this flat recruitment.
- Item prices, emblems, and reward item identities.

## Decisions

1. The cap checks run before the purse, so a refused roster does not spend.
2. Twelve copper is one silver at the 12-to-1 rate already used for the company's coins.
3. The same 12 applies to a combatant and to a specialist. The candidate list's separate price is not folded in.

## Risks / Trade-offs

- A company that hires eight people spends 96 before any match fee. Tests that assumed a free roster now include that spend.

## Migration Plan

No data migration. No format page: no field changes.

## Open Questions

- None for this slice. The 12-copper check and the spend were read before this change.
