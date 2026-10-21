# Proposal

## Why

The character view's chain walk follows the next nodes until it meets a node whose record is missing, or whose flags carry both `0x12000000` with a negative state word, and returns that node. Exarust clears and destroys the view but does not model the walk.

## What Changes

- A node without a record stops the walk.
- A node whose flags carry both `0x12000000` and whose word is negative stops it.
- Any other node follows its next node.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the character view's chain walk.

## Impact

- `exa_ui::menu`, from `first_settled_view_node`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: no published percentage moves; `TCharacterView` is not in the largest-components table. The headline stays 14.4% and 5.7%.
- Evidence: the chain walk: the next node, the record, the record's flags against `0x12000000`, and the state word against 0. No new corpus files.
- Out of scope: the rest of the character view.

## Phase

Phase 4, UI. The character view's chain walk.
