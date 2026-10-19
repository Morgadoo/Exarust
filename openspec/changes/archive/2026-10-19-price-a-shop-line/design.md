# Design

## Context

`Company::trade` already refuses an empty pair and a payment the purse cannot cover, and it records a payment as spent or a receipt as earned. The shop does not yet turn a line into copper. Each line is a 16-bit word: the low fourteen bits are a count, and the two high bits select the unit.

## Goals / Non-Goals

**Goals:**

- Price one line at 1, 12, 144 or 1440 copper per count, from no mark, the lower mark, the higher mark, or both marks.
- Sum a buy basket. Quarter the same sum, dropping the remainder, for a sell basket.
- Confirm a trade from those two totals.

**Non-Goals:**

- Packing a copper amount back into a word.
- The description card, the shop window, and stock rotation.
- Emblems, a candidate's listed price, and reward item identities.

## Decisions

1. The unit is chosen by the two high bits together. Both marks are 1440, which is ten gold at the 12-to-1 rate. The higher mark alone is one gold. The lower mark alone is one silver. No mark is one copper.
2. The quarter applies to the basket sum, not to each line. Two lines of 3 sell for 1.
3. `trade` stays the purse check. The new call prices the two baskets and then uses that check.
4. A missing model section is not represented here. The caller passes the words it already read.

## Risks / Trade-offs

- A basket sum uses a 32-bit wrap, matching the unsigned total the sale shifts. The tests stay inside that range.

## Migration Plan

No data migration. No format page: no field changes.

## Open Questions

- None for this slice. The marks, the four units, and the sale's shift of the basket sum were read before this change.
