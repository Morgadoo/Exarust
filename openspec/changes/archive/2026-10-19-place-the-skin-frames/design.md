# Design

## Behaviour

The place is x, y, width and height in pixels. The first switch reads style & 0x30: 0x10 appends a frame from x, y to x + width, y + height with the 0x9c then 0xb8 texture; 0x20 appends the same rectangle with 0xb4 then 0x9c. The second switch reads style & 0xc0: 0x40 appends a frame from x + 1, y + 1 to x + width − 1, y + height − 1 with 0x9c then 0xb8; 0x80 appends the same inset rectangle with 0xb4 then 0x9c. Any other value in either group appends nothing.

The texture numbers are byte offsets into the skin's texture table and are kept as such because their names are not established. The frame helper expands each rectangle outward by half a pixel and draws two concentric four-segment strips, one unit apart; that geometry is not modelled here.

## Open questions

The texture slots' names and the helper's strip order are not established.

## Docs

This method is 487 bytes. `TGUIDrawSkin` is 38%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
