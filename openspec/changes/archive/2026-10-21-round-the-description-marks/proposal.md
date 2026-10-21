# Proposal

## Why

The item card's weapon and apparel sentences turn a quality or condition
byte into a mark out of 100 by dividing it by 2.55. Exarust dropped the
fraction; the original converts with the processor's rounding (nearest,
ties to even), so a byte of 129 is a mark of 51, not 50, and joins its
sentence the other way around the half.

## What Changes

- Round the description marks to the nearest, ties to even.

## Capabilities

### New Capabilities

### Modified Capabilities

- `inventory-pane`: the weapon sentence's marks.

## Impact

- `exa_items::card` (`scale_mark`), with a test at the boundary.
- Evidence: REA on the weapon and apparel sentence builders: every
  division by 2.55 is followed by a rounding conversion, not a truncating
  one.
- Out of scope: the rest of the sentence builders, already modelled.

## Phase

Phase 4 (items), `docs/PLAN.md` section 7 item 2.
