# Design

## Dying tick

After the tone loses 0.003 and the pin gain loses 0.0003, `dying_tick` calls `walk_collapse`. That reads the measured chest and its fast velocity. With no chest, it returns. The stagger uses centimetres: the chest and the velocity times 100, and the step length from the scale and the leg extension. The scale root is the square root of the body scale. The tone is the one just written. If a step is already running, the stagger writes nothing. Otherwise `Ragdoll::walk_stagger` takes the target divided by 100, the frame anchor or the chest's horizontal place, and the current ankles as the spots. The walk marks the step running, so the next tick does not start another.

## Open questions

- The dropped weapon at death is not in this tick.

## Docs

The combat-rule figure stays 27.3%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
