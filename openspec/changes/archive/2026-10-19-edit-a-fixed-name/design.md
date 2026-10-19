# Design

## Edit

When `fixed` is set, `item_sentence` does not pick a phrase. A name shorter than three letters is returned with the same roll. A longer name finds the first 0x10 byte. That byte and the one after it are removed by sliding the tail left by two and shortening by two. When the pair does not fit before the last two bytes, those last two bytes are dropped instead. If the first remaining byte is at least Z, a lowercase a, e, i, o, or u takes "An ", and any other byte takes "A ".

## Open questions

- The apparel sentence is not in.

## Docs

`docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate `TGUIItemContainer` at 24%. The published headline stays 14.4% and 5.7%.
