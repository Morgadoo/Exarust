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
- Evidence: the 141-byte signature match: the query compared with the id, the record lookup, the binding word, the four signature bytes per entry compared with the record's bytes, and the `0x1000` mask. No new corpus files.
- Out of scope: the lookup itself.

## Phase

Phase 5, character creation. The signature match.
