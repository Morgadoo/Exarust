# Proposal

## Why

A restored capture publishes a shape and stops. The shape is not applied, and a focus is not invoked before the hand words.

## What Changes

- Shape 0 shows the default image when it differs, marks the default, and stamps the clock.
- Shape 1 and up use the record one before that index. A different non-zero image is shown. A hook on that record runs. The same image is left alone.
- A shape past the table changes nothing.
- A focus is invoked once before the hand words. A session mark that is not already 0 becomes 0.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the cursor shape table and the focus call after the lists.

## Impact

- `exa_game::menu`, from `MatchGate::begin_match`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the shape records and the focus call order. No new corpus files.
- Out of scope: the focus method body, and the rest of the session-mark leave path.

## Phase

Phase 7, arena career. The shape and the call, not the whole cursor.
