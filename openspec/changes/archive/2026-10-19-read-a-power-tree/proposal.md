# Proposal

## Why

Phase 6 can price a power, but it cannot read the tree that places those powers. The file stores each node and then shifts every later node by the first node's place.

## What Changes

- A power tree refuses a file whose first word is not the power magic.
- The node count is the word at byte 52. Each node carries a power id, two more words, an x, a y, and 16-byte links.
- A link count below 1 stores no links. Every node after the first subtracts the first node's x and y.
- A file that ends before the count is an error.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the layout of a power tree.

## Impact

- `exa_formats::power_tree`.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the tree reader. The corpus step reads installed `.pwr` files when they are loose and `EXANIMA_DIR` is set, and skips otherwise. Tests use synthetic bytes.
- Out of scope: drawing the tree, learning a node, and the text library after the nodes.
