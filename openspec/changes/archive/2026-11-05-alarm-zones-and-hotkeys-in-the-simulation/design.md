## Context

Follows `hits-in-the-simulation` and `repeat-scripted-grid-runs`. No `docs/formats/` page is involved.

## Decisions

- The zone check keeps its order after the restore, now inside `SimFrameStart`'s `Early` set.
- The hand chain is not moved yet: `SimFrameStart` runs only on frames that tick, so the hand's input edges (a click's `just_pressed`) must first be kept across frames without a tick, and its cursor ray, read from the window and the chase camera, handed over by the frame; the chain's item drag turns the held body and reads it again in the same frame, and the object clicks it sends are read in the same chain.

## Risks / Trade-offs

- The alarm's sounds and state reach the frame after the step.

## Open questions

- None for reverse engineering.
