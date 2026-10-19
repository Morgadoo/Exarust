# Proposal

## Why

Phase 7 draws a power-tree node, but it does not copy a known power into the tree or place the two edge points of a thick link.

## What Changes

- A node's mark is the last known pair whose id, with bits 20 to 23 cleared, equals the power.
- The first node sets bit 0x10000. An already opened mark keeps bit 0x4000 and sets bit 0x10000. Any other mark clears bit 0x4000 and opens from its links unless the top nibble is 7.
- A thick joint places two points around the middle of three, one to each side of the chord, at the given width.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: learned marks on a power tree, and the thick joint of a link.

## Impact

- `exa_formats::power_tree`.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the mark copy and the thick-joint pair. No new corpus files. Loose `.pwr` files stay skipped when `EXANIMA_DIR` is unset. Tests use synthetic nodes and points.
- Out of scope: charge-up, the other effect meshes, and the fast sine polynomial.
