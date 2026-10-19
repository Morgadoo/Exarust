# Proposal

## Why

A bank select counts an install and does not run it. The install stores an index and shows a record.

## What Changes

- A ready install with an index below 1 clears a current index, shows a fallback when it differs and is not zero, sets an armed byte, and stamps the clock.
- An index of 1 or more that matches the sign-extended current index returns.
- Any other index stores itself, shows the record one before it when that value differs and is not zero and a block word is 0, and runs a hook on that record.
- An index past the table stores the index and does not show. The platform call is counted.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the install body after a control-bank select.

## Impact

- `exa_game::menu`, from `MatchGate::begin_match`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the install's index compare, the record stride, and the show gate. No new corpus files.
- Out of scope: drawing through the platform call, a world update of the side, and the non-zero argument path.

## Phase

Phase 7, arena career. The install body, not the platform cursor.
