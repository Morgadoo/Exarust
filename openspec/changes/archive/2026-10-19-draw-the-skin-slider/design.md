# Design

## Behaviour

Let the place be x, y of size w by h, with inset i, thumb size t and fraction f.

Style 0 draws the track at x + 2, y + 2 with width w − 4 and height h − 4. The thumb offset n is round-to-nearest-even((w − 2i − t) × f). The thumb is drawn at x + i + n, y + i with width t and height h − 2i. The translation moves by −n before the thumb and by +n afterwards. The two thumb booleans are flag bit 0x10 and false.

Style 0x10000000 begins, fills x + i + 1, y + i + 1 with width w − 2i − 2 and height h − 2i − 2, binds the skin's 0xe0 texture, and draws a bar from x + i + n at y to x + i + n + unit at y + h, where n is round-to-nearest-even((w − 2i − unit) × f). It then ends.

Any other value of the style's two bits draws nothing.

## Open questions

The thumb size at 0x40, the fraction at 0x38 and the paint pointer at 0x48 are named only by offset. The four helpers' bodies are not in.

## Docs

This method is 639 bytes. `TGUIDrawSkin` is 47%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
