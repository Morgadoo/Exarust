# Design

## Behaviour

The angle is `(slot − 3.5) × (2π / count) + 1.0 − turn`. The cosine and sine come from the engine's paired routine, whose first output both ring routines use for x.

For each axis the scale is `offset_a × (unit × trig) × offset_b`, rounded to the nearest even integer with `cvtss2si`, then multiplied by the turn and added to the centre. The unit is a global, four at rest but changed by the screen, so it is a field.

The centre x is `width / 2`, less the unit when the scrolled byte is set and the page at `0x1c0` is positive. The centre y is `height / 2`, less three units when the page is 0.

## Model

`skill_ring_position(slot, ring)` returns the two floats. `count` must be positive, as the caller's loop guarantees.

## Open questions

The sine/cosine routine itself is not reimplemented; the platform's is used.

## Docs

This method is 333 bytes. `TSkillUI` moves from 2% to about 4%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
