# Design

## Drop

`release_hands` takes each filled slot. The loose weapon copies the item's own mobility, sets friction to 0.75, and sets the drag to that mobility widened and multiplied by 0.175. The striker flags become 0. The velocity is the one the weapon already had. The count, both slots, the class word and the primary are cleared. `start_collapse` calls it when the hands are passed. `dying_tick` calls it once, on the first tick, when the hands are still on the fighter. A later tick does not drop again.

## Open questions

- The loose weapon is not yet a free body in the solver. It keeps the release values only.

## Docs

The combat-rule figure stays 27.3%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
