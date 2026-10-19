# Proposal

## Why

A clear named check still treated every listed record as one flag. The gate walks those records.

## What Changes

- When the named check is clear, the first record whose kind is 0x10010a or 0x10010d and whose link is the world counts an open. A later record is not visited. A different kind or a different link does not open.
- A set named check still opens without a matching record.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the row walk inside the cursor pick.

## Impact

- `exa_game::menu`, from `MatchGate::run_gate`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the compares of the two record kinds and the link against the world, after the named check returns clear. No new corpus files.
- Out of scope: the pick raycast, the named-check body, and the open callee.

## Phase

Phase 7, arena career. This walk, not the raycast.
