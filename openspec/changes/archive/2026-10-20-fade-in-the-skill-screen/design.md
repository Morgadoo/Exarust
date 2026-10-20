# Design

## Behaviour

Bit 1 of the state byte gates the block. With the deadline `0x164` at or below the clock, the state byte becomes 4. On page 2 the draw sets alpha and sound to 0; any other page calls the close slot with 0 and ends.

With time left, the sound is `(deadline − clock) × sound / delay`, where the sound is the value the opening fade produced and the delay is the global used by the show deadline. The result is stored at `0x17c`; the alpha at `0x178` is untouched by this block.

## Model

`skill_screen_fade_in(state, deadline, clock, sound, delay, page)` returns the new state, the new sound, whether the close ran, and an alpha only for the page-2 closing case.

## Open questions

The draw calls after the fade are not in.

## Docs

This slice is part of the draw. `TSkillUI` stays at 32% until the whole draw is modelled. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
