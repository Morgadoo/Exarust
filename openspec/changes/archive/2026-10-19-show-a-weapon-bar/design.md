# Design

## Rows

`weapon_bars` takes the four bytes in the order slash, crush, pierce, thrust. A byte of 5 or less is absent. A higher byte is that integer times 0.05, through a double, stored as a float.

## Open questions

- The icon picture and the apparel rows are not in.

## Docs

`docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate `TGUIItemContainer` at 10%. The published headline stays 14.4% and 5.7%.
