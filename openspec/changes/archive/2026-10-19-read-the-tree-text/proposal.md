# Proposal

## Why

Phase 6 can read a power tree's nodes, but not the text chunk that follows them. Each string is a length word and the bytes after it.

## What Changes

- A text chunk after the nodes stores each string as a length word and the bytes that follow it, at that string's offset.
- A length below 1 is empty. A tree with no text chunk has no strings.
- A text chunk that ends after its first word is an error.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the text chunk of a power tree.

## Impact

- `exa_formats::power_tree`, the reader the nodes already use.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the text reader after the nodes. No new corpus files. Loose `.pwr` files are still skipped when `EXANIMA_DIR` is unset. Tests use synthetic bytes.
- Out of scope: drawing the tree, and learning a node.
