# Design

## Behaviour

With the place at x, y of size w by h and inset i, the rectangle is x + i, y + i, w − 2i, h − 2i. The method makes one virtual rectangle call with that rectangle and the two booleans from flag bits 0x10 and 0x40. When bit 0x40 is set it then calls the cornered overlay at the same rectangle. No other work happens.

## Open questions

The virtual body and the overlay's two texture binds are not in.

## Docs

This method is 204 bytes. `TGUIDrawSkin` stays 44%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
