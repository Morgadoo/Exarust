# Proposal

## Why

The power tree's selection validates a new word against the special roots, the table and the current and previous selections, then stores the focus and stamps it. Exarust does not model it.

## What Changes

- With neither word positive the base result returns.
- Mode 3 lets a special word 0x1000000 or 0x2000000 on either side select the new word and return 3.
- Otherwise the candidate is zero for bit 0x400000, the low 16 bits for a word in 1..0xfffe with a valid table entry, else zero.
- The new word is stored; an equal last focus returns 0, an equal previous selection returns 3, and a fresh candidate stores the focus, updates the previous when positive and stamps the clock.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the power tree's selection.

## Impact

- `exa_game::thaum`, from `select_tree_node`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 248-byte selection method's mode guard, table test and three result paths. No new corpus files.
- Out of scope: the node table and the base call.

## Phase

Phase 5, thaumaturgy. The tree's selection.
