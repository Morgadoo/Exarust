## Context

The original detects a weapon hit inside the solver (the weapon body's spheres against a limb mesh at the tick's particle poses), stores it as the striker's pending hit, and its post-step deals it the same tick. Exarust's held weapon bodies already classify their pair-stage contacts each tick; the bodyless strikers' sweep and the damage ran once a frame.

## Decisions

- Frames come from the bodies' particles at the tick's end (`Part::drawn(1.0)`, what the frame draws at a fraction of 1), not the propagated scene: both worlds then see the same frames, and the bridge carries nothing new.
- Blows are dealt first in the next tick rather than inside the post-step: nothing touches a body between the two, and frame-sent blows keep their "next tick" rule.
- The chain stays in one ordered post-step set after the classifier and the impacts.

## Risks

- The duel changes against the previous build (hits at step resolution, more contacts reach the damage routine since a pending clash clears every tick); the documented corpse scene's corpse lands elsewhere, so its drag point was re-aimed.
