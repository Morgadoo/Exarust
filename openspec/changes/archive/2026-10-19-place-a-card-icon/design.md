# Design

## Place

`card_icon` takes the stats flag, the span, the content, the picture width, the icon size, and the unit. A plain card uses `(span - icon) / 2` and `(content - icon) / 2 + unit * 4`. A stats card uses `unit * 13 - (icon >> 1)` and `unit`. When `unit * 11` is below the picture width, the horizontal place adds `picture - unit * 11`. The stored icon size is 160 and the stored unit is 4.

## Open questions

- Description phrases are not in.

## Docs

`docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate `TGUIItemContainer` at 14%. The published headline stays 14.4% and 5.7%.
