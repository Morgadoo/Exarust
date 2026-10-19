# Design

## Behaviour

With the place at x, y of size w by h, inset i, unit u and the flags at 0x1c, the method prepares once.

When flag bit 0x40 is set:

- The inner fill is [x + i + 2, y + i + 2, w − 2i − 4, h − 2i − 4] with texture slots 0x74 and 0x80.
- The border is [x + i + 1, y + i + 1, w − 2i − 2, h − 2i − 2] with slots 0x7c and 0x88.
- Slot 0x6c is bound and the outline is [x + i, y + i, w − 2i, h − 2i].
- The corner detail runs at [x + i + 1, y + i + 1, w − 2i − 2, h − 2i − 2] with mode 0xa0 and the unit.

Otherwise one virtual rectangle runs at [x + i, y + i, w − 2i, h − 2i] with the first boolean from flag bit 0x10 and the second false, and the inset height as the extra argument.

## Open questions

The four texture slots' names and the helpers' bodies are not established.

## Docs

This method is 698 bytes. `TGUIDrawSkin` is 50%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
