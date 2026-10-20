# Proposal

## Why

The creation view stores an appearance scale from the object's source value, and with a special flag rebuilds and draws its label. Exarust has the view copy, attach and size but not the scale apply.

## What Changes

- The scale is the source times 0.17 plus 0.9, stored at the child and its link.
- With the special flag, a clear global byte runs the global call.
- A missing label is built and the label is drawn.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the scale apply.

## Impact

- `exa_game::create`, from `apply_creation_scale`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 21% to 22%.
- Evidence: the 307-byte `0x10012d160`: `0.17` at `0x100315e18` and `0.9` at `0x100315e10`, the source `0x38`, the stores at `0x1a0` and `0x158 + 0x50`, the `0x100000` flag on `0x1c`, the `0x1002b3d60` guard and `0x1000cf330`, the label at `0x198`, the build from `0x1b0` and `0x190 + 1`, and the draw `0x1000cd390`. No new corpus files.
- Out of scope: the global call, build and draw bodies.

## Phase

Phase 5, character creation. The scale apply.
