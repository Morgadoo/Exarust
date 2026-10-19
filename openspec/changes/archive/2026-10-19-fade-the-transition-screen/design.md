# Design

## Behaviour

When field 0x178 is not 2 the method only calls the base draw. When it is 2, the method sets blend mode 0x203, takes the skin colour's three bytes and field 0x177 as alpha, selects the font and begins. It assigns the fixed string "TO BE CONTINUED" and centres it: x is (field 0x28 − 15 × glyph width) >> 1 and y is (field 0x2c − glyph height) >> 1, both logical shifts on the 64-bit difference. One draw follows and then the base draw.

## Open questions

The font object and the base draw are not in. The page field at 0x178 is named only by offset.

## Docs

This method is 260 bytes. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
