# Proposal

## Why

Phase 4 equips by occupancy, and the inventory windows are still missing. `docs/PLAN.md` §7 names them as the remaining inventory work.

## What Changes

- An inventory pane holds icons at pixel positions.
- A drop from outside adds the icon at the cursor minus the grab, adds its bulk, and clamps it inside the pane.
- A drop from the same pane or the linked pane only moves it. A linked move is not refused for capacity.
- An oversized item, a bulk that does not fit, and a container dropped onto its own pane are refused.
- Ending the drag removes the icon. The hit is the top-most icon whose rectangle holds the cursor.

## Capabilities

### New Capabilities

- `inventory-pane`: placing, moving and hitting icons.

### Modified Capabilities

## Impact

- `exa_items::pane`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the item-panel notes (accept, drop, drag ended, hit test). No format field changes, so the corpus step is a skip.
- Out of scope: the icon picture, the opaque-pixel test, and the description card.
