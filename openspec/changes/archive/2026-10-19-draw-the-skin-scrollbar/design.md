# Design

## Behaviour

Style 0 draws the track from x + 1, y + 2 with width w − 2 and height h − 4. The offset n is round-to-nearest-even((w − 2i − t) × f). The thumb is x + i + n, y + 1 + i, with width t and height h − 2 − 2i. The translation moves by −n and back, and flag bit 0x10 is the thumb's first boolean, with the second false.

Style 0x10000000 begins, fills x + i + 1, y + i + 1 with width w − 2i − 2 and height h − 2i − 2, binds the 0xe0 texture, and draws a bar from x + i + n at y to x + i + n + unit at y + h, where n is round-to-nearest-even((w − 2i − unit) × f). It then ends.

Any other style draws nothing.

## Open questions

The helpers' bodies are not in. The difference between this slider and the skin's other slider is only visual.

## Docs

This method is 588 bytes. `TGUIDrawSkin` is 52%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
