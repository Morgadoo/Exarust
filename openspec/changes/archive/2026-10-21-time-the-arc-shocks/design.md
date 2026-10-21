# Design

## Shock timing

`shock_reaction(ready, now, yellow, strength, dying, roll)` is the rule;
`shock_fighters` keeps the next shock time per fighter in a `ShockReady`
component, and remembers the times it set in the same tick for fighters
that had none yet, so a second arc in one tick is refused as in the
original. The draw is the engine's 32-bit generator (the effects' `Roll`),
`(state·200) >> 32`.

## Barricade knocks

The shove returns one knockback per part; the live tick applies the
knockback step once (every part's targets the same point from the chest)
and folds the energy losses in order with the 0.25 test
(`energy_after_knocks`).

## Open questions

- What the original's character kinds behind the shock routine's other two
  branches are (record bits 0x1000 and 0x100): not needed until the shock
  reaction itself is ported (REA, the character record's flags).

No `docs/formats/` page is concerned.
