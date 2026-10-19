# Design

## Owner

`release_hands` still builds the loose weapon. After the mobility, friction, drag and striker clear, the weapon's owner becomes its own id. Each child copies that owner, and the copy walks every descendant. The weapon is removed from its parent's list when that list contains it; a missing id leaves the list, and no parent stays none. The root walk writes the weapon's id onto the weapon and every descendant and counts each part it visits. The weapon's iteration count becomes 2 plus that count, minus 1. A child's own iteration count stays. The link count becomes 0 and the awake byte becomes 0. A grip at or above 0 becomes -1. A negative grip stays. A part sets its mark to -2 only when it has a child, its object type's low twelve bits are 0x89, and object bit 0x400000 is set. A part with no child keeps its mark. `dying_tick` already calls `release_hands` once.

## Open questions

- The loose weapon is still not a free body in the solver.
- The striking word is still cleared outright. The original flag word only loses bits 0, 16 and 17, including on each child. That mask is not this slice.
- The hand bodies' own drag, written before the weapon release, is a different object. It stays where the previous slice put the weapon drag.

## Docs

The combat-rule figure stays 27.3%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
