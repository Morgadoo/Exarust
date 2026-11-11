## Context

Exarust aims the attack starters once per fixed step, before the AI and
the character ticks run, from the combat target the attack stage copied in
the previous tick. The original reads the target as the starter runs.

## Decisions

- The duellist re-resolves its target right before its starters, the same
  way the attack stage does, and aims again only when the target changed,
  so a step without a change keeps the fixed step's aim exactly.
- The fighters in combat are taken once at the start of the duellist tick,
  after the AI tick. An attack started earlier in the same duellist tick
  is not seen, as in the fixed step's aim.
- Death ends a running parry before the body goes limp, while the hands
  still carry the arm bit.

## Open questions

- The brain lookup of the attack stage finds a brain only on the character
  root; scene brains live on the body, so their target is the duellist's
  foe in combat. Both paths keep that resolution.
