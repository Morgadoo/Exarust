# Proposal

## Why

Phase 7 lists every match stake as the raw template dword. That dword is a coin count, and a tier multiplies the copper by one more than the tier. The board still charges the raw word.

## What Changes

- A stake dword is copper, plus silver times 12, plus gold times 144.
- The recorded fee and prize are that copper. Tier 0 charges it.
- A higher tier multiplies both the fee and the prize by one more than the tier.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the special board's copper, and the tier's stake.

## Impact

- `exa_game::career`, the board and the entry the company already uses.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the template dword unpacked at 12 to 1, then multiplied by one more than the tier before it is paid or received. No format field changes, so the corpus step is a skip.
- Out of scope: emblems, reward item identities, a candidate's listed price, and which key fits a door.
