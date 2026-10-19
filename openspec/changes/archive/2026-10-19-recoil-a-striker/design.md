# Design

## Recoil

`recoil` always sets strength and the stall rate to 0.5 and clears the released bit. A class word of exactly 0x90 that already has the continuation bit, or whose clock widens above the double 0.4 while a counter starts, writes nothing more. A float clock of 0.4 widens above that double, so it takes the skip when the counter is set. The next lower float does not. Any other swing sets stalling, cleaving and recoiled. The end widens the drive, multiplies by the double 0.1, narrows, multiplies by 400, and truncates toward zero, then adds now and 200. The stance blend is 40. Both arm stiffnesses are 0.2, passed with a held scale of 1.

## Follow-through

`follow_through` uses the same skip and returns nothing when it matches. Otherwise the scale widens the speed, multiplies by the double 0.15, widens the recoil and the gain around that product, narrows, and floors a negative result at 0. A body zone that is not strictly above the epsilon sets strength 0. A larger zone uses its square root times the scale. An edge uses one minus the zone, times the scale, plus the zone, all in single precision. Strength above 1 becomes 1. The stall widens one minus the strength and multiplies by the double 0.05. Flags gain stalling and cleaving. The end truncates the scale times 400 and adds now.

## Open questions

- The caller still chooses the gain. A large attacker, a body-of-weapon hit and an edge each pick a different one, and that choice is not in this function.
- Size at or above 1.4 is a caller gate on recoil. This function does not read the attacker's size.

## Docs

The combat-rule figure stays 27.3%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
