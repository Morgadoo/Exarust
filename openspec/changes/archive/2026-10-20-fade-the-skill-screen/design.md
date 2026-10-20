# Design

## Behaviour

The state byte is the low byte at `0x158`. Byte 4 is the closing state and returns zero. Otherwise bit 0 set and a deadline `0x160` at or below the clock clears the bit and returns 1. With bit 0 set and time left, `t = (deadline − clock) × 0.002` in double precision, `x` is the float `1 − t`, then `x × (2 − x)`. The alpha is `x` clamped to at most 1. The sound value is `x × (2 − x)` again and is passed to the sound call with code `0x23`. Bit 0 clear returns 1.

## Model

`skill_screen_fade(state, deadline, clock)` returns the alpha, the sound value and whether the bit was cleared. The rest of the draw is out of scope, so the component's rating does not change.

## Open questions

The rest of the draw is not in.

## Docs

This slice covers the draw's opening. `TSkillUI` stays at 32% until the whole 2,955-byte draw is modelled. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
