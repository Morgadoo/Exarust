# Design

## Behaviour

The handler first returns 0 for state bit 1 with a page below 2. The base hit `0x10002d270` runs; a form that does not own the point returns that result, and so does a base that is not −1 when `0x100028a20` refuses.

The point is transformed, then the centre is half the width and half the height less three units. On page 2 a point between 16 and 8 units above the bottom and within 6 units of the centre returns `0x10000`. A state byte of 4 returns the base result.

A positive high selection uses the node point from slot `0x1c8`, raised by 6 units and passed through `0x1002275a0` with 41 units. A point within four units vertically gives 1 for an offset between −13 and −4 units and 2 for one between 4 and 13 units; anything else is −1.

A positive low selection uses its node point, lowered by 8 units and passed through the same call. Within eight units vertically the slot is `round(4.5 + offset / (10 × unit))`, returned when it is 1 through 8 and −1 otherwise.

With no selection the fallback slot `0x1c0` runs.

## Model

`hit_skill_point(input)` takes the state, page, base, ownership, guard, transformed point, size, unit, selections and their adjusted node points, and returns the code.

## Open questions

The base, guard, radius and fallback bodies are not in.

## Docs

This method is 857 bytes. `TSkillUI` moves from 20% to about 25%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
