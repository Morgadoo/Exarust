# Proposal

## Why

The power tree's text chunk (`0xABCD0000`) holds its learning prompts as offset-addressed length-prefixed strings. Exarust now models the tree's behaviour but not this loader.

## What Changes

- A 16-byte header whose last word is the count.
- `count + 1` little-endian u32 blob offsets, a u32 blob size and the blob.
- Each string is a u32 length followed by its bytes.
- A short header, an offset past the blob, or invalid UTF-8 returns nothing.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the tree text chunk.

## Impact

- `exa_game::thaum`, from `TreeText` and `parse_tree_text`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TThaumTree` becomes **18%** in the source rating. The headline rises slightly.
- Evidence: `0x10026f6c0` (the header and offsets read), `0x10026f790` (the length-prefixed fetch). No new corpus files.
- Out of scope: the tree's create/reset and the rest of the loader.

## Phase

Phase 7, thaumaturgy. The tree text chunk.
