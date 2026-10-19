# Design

## Pick

`pick_phrase` keeps rows whose low and high include the value. An empty list returns the same roll and no text. Otherwise the roll becomes `state * 0x08088405 + 1`, and the index is the high half of that times the candidate count. A shared word longer than three letters, taken from every space that is not the string's last character and compared without letter case, drops the text but keeps the new roll. A phrase that starts with a comma is joined to a dot. Any other phrase is a space, the phrase, and a dot.

## Open questions

- The sentence built around the phrase is not in. The retail phrase tables are not copied.

## Docs

`docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate `TGUIItemContainer` at 20%. The published headline stays 14.4% and 5.7%.
