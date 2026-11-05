## Context

The original keeps one step record on the body: the stepping byte, the
balance-step byte, the step target, the step kind, the start clock, the
landing count, the queue word and vector. The balance routine, the
knockback, the AI and the player's behaviour all write it; the balance
routine, the feet routine, the pins, the locomotion gate, the lift and the
player's dispatcher read it. Exarust kept the player's step apart from the
body and moved the character itself.

## Decisions

- The player's `StepRecord` stays as the dispatcher's view: it is read from
  the body before the dispatcher and written back after it, so the
  dispatcher's own tests run without a body. The start clock is the body's
  episode age before the game clock's now.
- The queued step's grid clamp: Exarust's body has no grid, so the
  behaviour clamps a taken target (queue below zero) on its next tick, by
  the line walk from the chest and the snap with no distance test, as the
  original's taker does in its pass.
- `Bearing::stepping` is the body's episode: the set-bearing keeps it, the
  movement stage copies last tick's, the balance routine refreshes it once
  this tick's step is known (the lift reads it after).
- The ground placement's pins and pin mean are not ported: the body tick
  takes both anew from the frame before the solver and the balance read
  them, and Exarust's pins are taken from the pose every pass.

## Open questions

- Whether the leg routine's anchored-foot rule reads the stepping byte (as
  the lift does) was not re-checked; it reads the same flag as before.
- The step length field the goal routine writes is read by the stance
  clamp in the original; Exarust's clamp uses its own footwork length.

## References

docs/PLAYER.md (Combat footwork), docs/re/motiledyn-map.md.
