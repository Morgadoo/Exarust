# Proposal

## Why

The creation screen searches a part tree for a part tagged `0xFCA0`, first among a node's attachments and then among its children. Exarust has several creation behaviours but not the search.

## What Changes

- A node with kind masked by `0x89` equal to `0x89` and a non-negative wanted count is searched for the tag.
- An attachment tagged `0xFCA0` matches.
- Otherwise the children are searched, and a leaf without a match is false.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the part search.

## Impact

- `exa_game::create`, from `has_creation_part`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` stays about 12%.
- Evidence: the 173-byte part search: the `0x89` kind mask, the wanted count, the attachment list with its `0xfca0` tag shifted 16, the child count and list, and the recursive call. No new corpus files.
- Out of scope: the part objects themselves.

## Phase

Phase 5, character creation. The part search.
