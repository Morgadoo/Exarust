# Proposal

## Why

Phase 7 lights a shell from its fade. The second light's two values, and the index upload that flag takes, are not in.

## What Changes

- A resting second light stores 0.1 and 0.75.
- While opening, a shown value under 0.25 stores one tenth of its times-4 product with 2 minus that product, and 1 minus that product times 0.25. A shown value of 0.25 or more stores 0.1 and 0.75.
- While closing, the first value is one fifth of 2 minus the absolute gap from 1.5 to the shown value times 3.5, and the second is the shown value times 0.75.
- A hold keeps the pair it was given.
- An index upload asks for 20 bytes plus 8 a channel, and a payload of the triangle count times the width. The target is 0x8893, the usage 0x88E4, and the access 0x88B9. A finish unmaps only when the buffer name is above 0.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the second shell light and the index upload.

## Impact

- `exa_game::thaum`, beside the shell light and the index room.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the second light and the element upload. No new corpus files. Tests use synthetic shown values and spans.
- Out of scope: issuing those buffer calls from the play renderer.
