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
- Evidence: the 307-byte scale handler: its `0.17` and `0.9` constants, the source value, the two stores, the `0x100000` flag on the object's flag word, the global guard and its update, the label, its build from the label base and the index plus one, and the draw. No new corpus files.
- Out of scope: the global call, build and draw bodies.

## Phase

Phase 5, character creation. The scale apply.
