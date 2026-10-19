# Design

## Page

`item_card` refuses an id below 1, a container, a missing model, and a model mark of 0. A map returns the map page and no value. Otherwise the value is the coin word shifted right by 2, or the whole word when the high bit is set. A weapon without the withheld bit is the weapon page. An armour byte from 1 through 6 is the apparel page. A shield without the withheld bit is the shield page. The rest are plain.

## Open questions

- The icon picture and the stat bars are not in.

## Docs

`docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate `TGUIItemContainer` at 8%. The published headline stays 14.4% and 5.7%.
