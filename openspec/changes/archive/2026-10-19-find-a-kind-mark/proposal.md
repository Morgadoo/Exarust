# Proposal

## Why

A picked item that is not taken counted a kind call for any non-zero id. The check searches a table.

## What Changes

- An id of 0 does not count. Bit 0x80000000 searches the high table. Else bit 0x40000000 searches the wide table. Otherwise the plain table is searched. A high or wide id drops those bits.
- The search is the signed binary search of the sorted keys. A found row counts when its span holds 0x20000. A missing key, a size of 0, a short span, or a span without the mark does not. A step of 0 stops the walk.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the kind-table lookup inside the cursor pick.

## Impact

- `exa_game::menu`, from `MatchGate::run_gate`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the table choice, the signed search of 16-byte rows, and the span walk for mark 0x20000. No new corpus files. The tables in tests are synthetic.
- Out of scope: the two calls after a hit, and the pick raycast.

## Phase

Phase 7, arena career. This lookup, not the raycast.
