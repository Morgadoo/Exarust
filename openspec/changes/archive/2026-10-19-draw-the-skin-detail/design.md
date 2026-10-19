# Design

## Behaviour

With the place at x, y of size w by h and inset i, the inset rectangle is x + i, y + i, w − 2i, h − 2i. The method prepares once and draws that rectangle once. Two booleans are read from the flags at 0x1c: bit 0x10 and bit 0x40.

When style bit 0x20000000 is set and bit 0x40 is clear, the horizontal helper runs at the original x, y, w, h with stack arguments −unit/2 and 0x50. When style bit 0x40000000 is set and bit 0x40 is set, or flag bit 0x80 is set, the vertical helper runs at the inset rectangle. Other combinations add nothing.

## Open questions

The two helpers' bodies are not in. The style bits' names are not established.

## Docs

This method is 321 bytes. `TGUIDrawSkin` stays 44%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
