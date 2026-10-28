## Context

The click handler turns toward a point beyond 70 cm, then reads the hit:
a placed object's usable section (none: the card; a lock: open → its
container, closed with the key and the reward due → the journal line and
100 or, for the lock subtype 3, 1000 experience; kinds 0 and 1: the
container; others: the card), a corpse's roles' scripts, or the player's
own body, which sets the HUD's show-until 5000 ms ahead before the click
callback. The double-click handler first runs the object helper (kind 0
consumes or takes the item and kind 1 takes it in hand while the used-mark
byte is clear, kind 5 pushes while its flag allows); only kind 1 returns,
otherwise a container section opens.

## Decisions

- The HUD keeps a `HudShowUntil` resource in virtual-time ms; the show
  rule adds `now < until`, as the original's.
- Exarust reads the double click on the second press's release, where the
  existing click handling is; the original takes it on the press.
- The double-click time is the system's; Exarust fixes Windows' default.
- The box test takes an `Affine3A` and sums as the point transform does.

## Risks

The click now picks under the cursor on every non-drag release outside
combat mode: one ray pick per click, not per frame.
