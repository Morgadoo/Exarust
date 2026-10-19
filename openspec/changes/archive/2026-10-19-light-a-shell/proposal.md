# Proposal

## Why

Phase 7 fades a shell by its stored size. The light that follows that shown value is not in.

## What Changes

- While opening, a shown value whose double is above 0.666 becomes one half. Any other becomes 0.75 minus the absolute gap from 0.75 to the shown value times 1.5.
- Any other state takes half the shown value.
- The light is one tenth of that.
- While closing, the gain becomes the shown value times 2.8. Any other state keeps its gain.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the shell light that follows the fade.

## Impact

- `exa_game::thaum`, beside the shell fade.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the shell light curve. No new corpus files. Tests use synthetic shown values.
- Out of scope: the second light's two channels, and uploading the GPU buffers.
