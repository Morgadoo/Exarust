# Proposal

## Why

The creation screen refreshes its label at most once per 300 clock units: it builds the label when absent and draws it. Exarust has the record, labels, scales, frame and part search but not the throttled refresh.

## What Changes

- A clock less than 300 above the last stamp does nothing.
- Otherwise the stamp takes the clock, and the label is built when no widget exists.
- The widget is drawn after the build.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the throttled label refresh.

## Impact

- `exa_game::create`, from `refresh_creation_label`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 13% to 14%.
- Evidence: the 271-byte `0x10012d020`: the clock `0x1004997c0`, the stamp `0x1004f0420`, the 300-unit guard, the widget at `0x198`, the label build from `0x1b0` and `0x190 + 1`, and the draw `0x1000cd390`. No new corpus files.
- Out of scope: the label build and draw bodies.

## Phase

Phase 5, character creation. The throttled label refresh.
