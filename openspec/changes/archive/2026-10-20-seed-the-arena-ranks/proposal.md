# Proposal

## Why

The arena rank roll uses the engine's MT19937 stream, separate from the `0x8088405` LCG used for every other draw. The LCG-only model cannot reproduce rank sequences.

## What Changes

- A standard MT19937 generator with the 624-word state, twist and tempering.
- The `unit()` draw `next_u32 · 2⁻³²`.
- `roll_instance_mt`, which advances the MT stream only when the template needs a rank roll.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the arena rank stream.

## Impact

- `exa_game::arena`, from `Mt19937` and `roll_instance_mt`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `XA_Arena` becomes **29%** in the source rating. The headline rises slightly.
- Evidence: the generator's tempering (constants `0x9d2c5680`/`0xefc60000`), its unit draw, and the rank roll. No new corpus files.
- Out of scope: where the arena seeds the MT state and how other systems consume it.

## Phase

Phase 7, arena. The rank stream.
