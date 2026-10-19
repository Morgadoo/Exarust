# Design

## Context

The board stores a fee and a prize for each match, and entering subtracts the fee. Those fields were copied as the raw template dwords. The purse and the stake use the same coin dword: the first byte is copper, the next is silver, and the following 16 bits are gold, at 12 to 1. Setting a tier multiplies the unpacked copper by one more than the tier and stores it back.

## Goals / Non-Goals

**Goals:**

- Read a stake dword as copper.
- Record each match's tier-0 fee and prize as that copper.
- Charge and pay `copper * (tier + 1)`.

**Non-Goals:**

- The random opponent count a flagged match draws.
- Emblems, reward item identities, and a candidate's listed price.

## Decisions

1. The board holds tier-0 copper, the amount a company pays or receives. The dword `0x100` is one silver, 12 copper. `0x300` is 36. `0x200` is 24. `0x30000` is three gold, 432.
2. Tier 0 multiplies by 1, so an entry with no tier keeps today's call.
3. The same multiplier applies to the fee and to the prize.

## Risks / Trade-offs

- A duel prize of 256 was the raw word. Its copper is 12. Tests that added 256 now add 12.

## Migration Plan

No data migration. No format page: no field changes.

## Open Questions

- None for this slice. The coin dword and the tier multiplier were read before this change.
