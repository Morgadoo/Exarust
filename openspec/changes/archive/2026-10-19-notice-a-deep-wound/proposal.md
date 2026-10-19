# Proposal

## Why

A deep lasting wound is taken off the health bars and nothing records that it was noticed.

## What Changes

- A lasting wound is noticed when it times 0.75, widened, times 0.2, is greater than 0.25 widened times 0.05.
- Bits 0x3daaaaab are not noticed. The next float is. A non-number is not.
- The hit path records that notice for a chest cut.

## Capabilities

### New Capabilities

### Modified Capabilities

- `combat`: noticing a deep lasting wound.

## Impact

- `exa_combat::damage::notices_damage`, called from the hit path.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and `docs/formats/OTHER.md`.
- Evidence: the damage routine's comparison after it applies health. No new corpus files.
- Out of scope: limb impairment, the state that knocks a character unconscious, and practice mode.

## Phase

Phase 4, the first duel. Noticing a deep wound.
