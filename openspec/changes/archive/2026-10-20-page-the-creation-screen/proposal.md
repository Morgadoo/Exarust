# Proposal

## Why

The creation screen pages forward when the index limit allows and selects a wanted index through the mode and refresh. Exarust has the mode toggle and list steps but not these two.

## What Changes

- Paging does nothing unless the maximum exceeds the index.
- Otherwise the mode becomes 1, the index rises by one and the refresh runs.
- Selecting uses the wanted index less one; an unchanged index does nothing, otherwise the mode becomes 1 and the refresh runs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the page and select.

## Impact

- `exa_game::create`, from `page_creation_next` and `select_creation_index`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 27% to 28%.
- Evidence: the 95-byte `0x10012fa50`: the max at `0x4f0c20 + 0x8` less one, the index `0x4f0370 + 0x254`, the mode `0x274` and the refresh `0x10012f950`. The 61-byte `0x10012fab0`: the wanted index at `0x38` less one, the current at `0x254`, the mode and the refresh. No new corpus files.
- Out of scope: the refresh body.

## Phase

Phase 5, character creation. The page and select.
