# Proposal

## Why

Phase 7 draws the line between two power-tree nodes, but not the icon, the frame or the glow on a node.

## What Changes

- An icon with no columns draws nothing. Otherwise the cell is the power's low byte, one less, across the sheet, and the square is centred on the node.
- A second byte of 1 uses a bias of 1. Any other uses the other bias.
- A frame uses the slot one less, four across and two down, at four fifths of the size.
- A wide picture drops the fraction of the view. A narrow one uses half.
- A glow is 32 spokes on a ring of 0.84 of the width plus the outer size, starting straight up.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the icon, frame and glow drawn on a power-tree node.

## Impact

- `exa_formats::power_tree`, beside the link curve.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the icon cell, the frame square and the glow ring. No new corpus files. Loose `.pwr` files stay skipped when `EXANIMA_DIR` is unset. Tests use synthetic points.
- Out of scope: learning a node, the thick-line quads, and the fast sine polynomial.
