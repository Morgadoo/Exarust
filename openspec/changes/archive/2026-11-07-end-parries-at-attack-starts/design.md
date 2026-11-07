## Context

Exarust's parry state (`ParryState`) holds the original's parrying flag
and countered-motion word; its hold (the hands' arm bit and the parry
grip link) is put on and taken off by the parry check in the body tick.
Exarust has no single motion-start routine.

## Decisions

- The starter's end is a command queued by the common attack start, read
  against the parrying flag when it runs (before the tick's parry check).
  Whether the clip left is an attack is read from the animator before the
  start; a chain always leaves an attack.
- The motion start's release when an attack is left is placed in the
  parry rule's tick: a fighter with no attack under way, playing no attack
  clip, not parrying and still countering a motion is released there (the
  countered motion is only set by an attack start once a parry has
  ended). The tick the attack ends is the tick its clip changes.
- The release touches the grips and hands only when a hand carries the
  arm bit: the parry grip link is enabled by the parry's hold alone, so
  disabling it otherwise changes nothing.
- The overhead and stab's stance re-choice also updates the clip the
  attack returns to when it was the old stance.
- The pins: the gains and the tick's goals are read once per iteration;
  each part pulls its own goals in the list order after its links and
  world links, before its foot contact.

## Open questions

- The heavy-claws hand mass write at three starters equals the equip
  pass's for unarmed claw races; an armed claw race would differ (none
  is armed in the shipped arenas). Left as is.
