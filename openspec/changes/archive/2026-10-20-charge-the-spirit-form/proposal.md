# Proposal

## Why

`TPwrSpiritForm` is the Mind power's spirit form: a value that charges to 1 over 30 s, holds, or fades to 0, publishing a strength and a smooth body fade. The whole `SG_Thaum_Mind` component is at 0%; this is its first slice.

## What Changes

- Charging raises the value by one per 30 s and clamps at 1, becoming held.
- Holding keeps the value at 1.
- Fading lowers it by one per 30 s and ends below 0.002.
- Fading publishes `1000·value`; otherwise the smooth value is `1 − (1 − value)²`, the strength `(1 − smooth)·rate + 1000·smooth`, and the vector scale `1 − value`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the spirit form.

## Impact

- `exa_game::mind`, from `SpiritForm`, `SpiritStep`, `step_spirit_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Thaum_Mind` becomes **2%**. The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: the spirit form's tick: its state, value, stamp and rate fields, the 30,000 ms double, the 0.002 double, the 1000 strength and the `1 − (1 − value)²` smooth blend. No new corpus files.
- Out of scope: the body writes and the hook at the tail.

## Phase

Phase 7, thaumaturgy. The spirit form.
