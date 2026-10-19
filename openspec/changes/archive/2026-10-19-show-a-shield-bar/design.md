# Design

## Rows

`shield_bars` stores weight as the byte times 0.01 and impact as the byte times 0.05. Both scales are doubles, then stored as floats. Encumbrance uses the impact scale and is absent when the byte is 0. When the flags share a bit with `0x06000000`, weight is the stored 1 and encumbrance is the stored 8, including when the encumbrance byte is 0.

## Open questions

- Rank, points, and description phrases are not in.

## Docs

`docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate `TGUIItemContainer` at 16%. The published headline stays 14.4% and 5.7%.
