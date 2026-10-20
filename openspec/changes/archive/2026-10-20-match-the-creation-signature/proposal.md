# Proposal

## Why

The creation screen matches a lookup id against a signature: the id itself, or a binding whose id and four bytes match. Exarust has several creation behaviours but not the match.

## What Changes

- A query equal to the signature's id matches.
- Otherwise the binding looked up by the query must have the signature's id and the signature's four bytes.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the signature match.

## Impact

- `exa_game::create`, from `signature_matches`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` stays about 10%.
- Evidence: the 141-byte `0x10012e0a0`: the query at `(%rbx)` compared with the id, the `0x100103db0` lookup, the binding word at `+0x14`, the four bytes at `+0x1bc + i × 12` compared with `+0xf + i`, and the `0x1000` mask. No new corpus files.
- Out of scope: the lookup itself.

## Phase

Phase 5, character creation. The signature match.
