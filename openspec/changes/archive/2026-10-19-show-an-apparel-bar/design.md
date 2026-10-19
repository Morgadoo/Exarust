# Design

## Rows

`apparel_bars` always stores coverage as the byte times 0.05. Impact, slash, crush, pierce, and encumbrance use the same scale, and a result that is not above 0.25 is absent. The compare is on the double, before the value is stored as a float.

## Open questions

- The icon picture is not in.

## Docs

`docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate `TGUIItemContainer` at 12%. The published headline stays 14.4% and 5.7%.
