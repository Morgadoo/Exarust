# Proposal

## Why

`TPwrForceKinesis_S` (4.1 KB) drives the Force domain's telekinesis. Its constructor computes a scale from the effect's speed and its activation handler hooks the owner, flags the target and releases on removal. Exarust does not model it.

## What Changes

- The scale is `(1/speed + 0.015)·0.5`.
- An event carrying bit 1 or 3 moves the effect to removal.
- Removal releases a held target and returns 2.
- The active state hooks the owner when it accepts, raises the target's flag and registers the frame callback.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the force kinesis effect.

## Impact

- `exa_game::thaum`, from `KINESIS_REACH`, `KINESIS_DAMPING`, `kinesis_scale`, `KinesisEffect` and `step_force_kinesis`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TPwrForceKinesis_S` becomes **10%** in the source rating. The headline rises slightly.
- Evidence: `0x100246ca0` (the 2.8, 0.5 and scale constants at `0x10033f100`, `0x10033e5e8`, `0x10033e600`, `0x10033f108`, `0x10033ede8`) and `0x100246200` (the `0xa` event test, the release, the owner hook, the target flag and the callback registration). No new corpus files.
- Out of scope: the 786-byte grab update and the 893-byte release body.

## Phase

Phase 7, thaumaturgy. The force kinesis effect.
