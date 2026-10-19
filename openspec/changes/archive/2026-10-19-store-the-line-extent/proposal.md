# Proposal

## Why

The line class stores five extents when it opens: twice the unit, the unit, twice the unit, twice the unit and four times the unit. Exarust stores only the first four.

## What Changes

- Opening a skin at unit 5 also stores an extent of 20.
- The value is four times the unit.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the fifth line extent.

## Impact

- `exa_ui::skin`, in `open_skin`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the opening method's fifth store. No new corpus files.
- Out of scope: the methods that read the extent.

## Phase

Phase 7, the interface. The store, not its readers.
