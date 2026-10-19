# Proposal

## Why

Phase 7 can read a power tree and its text, but nothing places a node or draws the line between two nodes.

## What Changes

- A node sits at ten times its place times the zoom, plus the pan, and rounds to the nearest even integer unless the view is dragged.
- A factor scales that point about the origin.
- A link with both marks at zero draws nothing. Otherwise each end leaves its node along the sine and cosine of its turn times a full circle, by the long or short reach.
- A zero source mark dims the line.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the line drawn between two power-tree nodes.

## Impact

- `exa_formats::power_tree`, beside the reader.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the node place and the link curve. No new corpus files. Loose `.pwr` files stay skipped when `EXANIMA_DIR` is unset. Tests use synthetic points.
- Out of scope: icons, frames, glows, the thick-line quads, the fast sine polynomial, and learning a node.
