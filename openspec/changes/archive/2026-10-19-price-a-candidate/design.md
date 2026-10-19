# Design

## Context

Recruitment of a made fighter costs 12 copper and checks the roster before the purse. A candidate on the hire list is priced when the panel opens, and that coin dword is what the hire spends.

## Goals / Non-Goals

**Goals:**

- Price a candidate from six skill levels and the shop words of the gear.
- Refuse a short purse before considering a full roster.
- Refuse a full roster or a second specialist without spending.
- Subtract the price and record it as spent when the hire joins.

**Non-Goals:**

- The flat 12-copper recruitment.
- Copying techniques, health, or a loadout onto the new member.
- The other emblem controls.

## Decisions

1. Half of the six-level sum stops at 5. One more than that is squared and multiplied by 12.
2. Each gear word uses the shop line price. Four fifths of that sum, to the nearest copper, is added. An integer gear total is never exactly halfway, so the nearest copper is `(4 * gear + 2) / 5`.
3. The purse is compared with the price before the roster caps. A cap that fails does not spend.
4. The flat recruitment stays as it is.

## Risks / Trade-offs

- A gear total that does not fit in a coin dword would be trimmed before the four fifths. The tests stay inside that range.

## Migration Plan

No data migration. No format page: no field changes.

## Open Questions

- None for this slice. The rank square, the gear sum, and the four-fifths step were read before this change.
