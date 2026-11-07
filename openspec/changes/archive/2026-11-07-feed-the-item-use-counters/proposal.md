## Why

PLAN Phase 5 (the physical body). The body tick (routine #49 of
`docs/re/motiledyn-map.md`) feeds two use counters that the item-effect
list's once-a-second run reads (the golem suit's worn tick, the gauntlets'
apparel tick) and spends. Exarust's list kept the counters, but nothing fed
them, so those ticks never stepped.

## Evidence

REA (Exanima.exe), the body tick's assembly: the movement's counter gains
0.0005 × the locomotion speed in the walk-or-run branch, 0.0025 a tick a
step runs after the balance routine (0.005 for a long step), and the
attack's counter 0.005 a tick of an attack clip in the attack branch; each
in double. The list's run takes one off each counter above one; an empty
list clears both.

## What Changes

- exa_physics: `Drive::item_use`, `set_item_use`; the muscle tune feeds the
  striding and attack counters, the balance routine the step's.
- exa_game: `item_effects::run_worn_effects` reads the counters from the
  body and writes back what the run left; a debug log of each run.
- docs/re/motiledyn-map.md.

## Impact

Only the worn and apparel item ticks read the counters, so no motion
changes; a body wearing a golem suit or gauntlets with an effect now steps
its item's byte as it moves and attacks.
