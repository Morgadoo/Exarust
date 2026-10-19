# Design

## Link

`stick` returns nothing unless the blow is armed and the bite is above 0.1. The depth is then capped at 1. Flags compare that float widened to a double with 0.4, so a float 0.4, which is slightly above the double, takes 0x211000. Strength is 12 times the capped depth. Smoothing is the caller's zone smoothing plus 0.5. The count converts 83 times the unit to a float, multiplies by the depth, truncates toward zero, and multiplies by 3. The span is twice the untruncated product. The formulas for a depth above 2 are present after the cap in the image and never run.

## Layers

The weapon layer is stored as 0xC0. `stuck_layers` adds bit 0x40 to the struck part, to its parent when there is one, and to each child. A missing parent stays missing.

## Open questions

- The lifetime unit is passed in. What the owner stores there is not re-read here.
- The plain link iteration that later clears the layers is not in.

## Docs

`docs/PLAN.md` and `docs/COMPONENT-MAP.md` keep the combat-rule figure at 27.3% and the published headline at 14.4% and 5.7%. The stick does not move the fresh sum off about 15.5%.
