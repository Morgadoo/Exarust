# Design

## Stagger

`stagger_step` returns nothing when a step is already running. Otherwise the length is the reach times the balance when the balance widens above the double 0.4, and the widened reach times the double 0.4 when it does not. A float balance of 0.4 widens above that double. The span squares the tone plus one half, in single precision, then multiplies by the length times the stored square root of the scale. The direction is the fast chest velocity times the reciprocal square root of (x² + z²) + y², and a zero sum leaves a zero direction. The target is the chest plus that direction times the span. Landings and the step kind are 0.

## Open questions

- The footwork that walks the target, and the pins that pull the chest toward it, are not in this function.
- The reach and the stored scale root are caller inputs. This function does not read a character record.

## Docs

The combat-rule figure stays 27.3%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
