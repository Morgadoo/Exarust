# Proposal

## Why

The item panel's geometry is modelled, but its mouse layer is not: the original decides between starting a drag, using or opening an item, and showing its card from the event's button bits, and it decides on drag end whether to cancel, snap back, transfer or remove. Those decisions are exact and testable.

## What Changes

- A mouse-down maps to no action, a drag start with the cell-centre grab, use, open or card.
- Right-click passes to the base form; a drag start needs drag permission and a hit; use/open and card need a hit.
- Drag end maps to no change, cancel, snap back, transfer to the linked panel, or remove.
- The grab offset after a drag starts is the cell's centre.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the item panel's mouse and drag-end decisions.

## Impact

- `exa_game::item_container`, from `MouseIntent`, `mouse_intent`, `drag_grab`, `DragEnd` and `drag_end`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TGUIItemContainer` becomes **25%** in the source rating. The headline rises slightly.
- Evidence: the item panel's click handler (the 0x2 right, 0x4000 drag, 0x8000 use/open and 0x2000 card branches, and the grab reset to the cell centre), its drop (cancel, snap back, linked transfer and removal) and its ghost draw offset. No new corpus files.
- Out of scope: the callbacks themselves, the popup placement, the item cards and the icon cache.

## Phase

Phase 7, items. The panel's mouse layer.
