# Proposal

## Why

The tree text's create and reset path sets the header before parsing: the `en` language id, flags `0x4e4`, kind 1, a zero marker and one empty string. Exarust parses the header but does not model the fresh chunk or read those fields.

## What Changes

- A fresh chunk carries language `0x6e65` (`en`), flags `0x4e4`, kind 1, marker 0 and one empty string.
- The parser reads the language word, flags and kind words, the marker and the count from the 16-byte header.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the fresh tree text and its header fields.

## Impact

- `exa_game::thaum`, from `new_tree_text` and the extended `TreeText`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TThaumTree` becomes **20%** in the source rating. The headline rises slightly.
- Evidence: the tree text's create (its writes) and its reset (count 0, one zero offset, blob size 4, one zero-length string). No new corpus files.
- Out of scope: the tree's draw.

## Phase

Phase 7, thaumaturgy. The fresh tree text.
