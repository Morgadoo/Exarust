# Design

## Behaviour

The label measures its text as the sum of the glyph advances. The place has x, y, w, h, an inset, a style word and a state word.

The horizontal style is the style word masked with 0x7000. Exactly 0x1000 puts the left edge at x minus the width minus the unit. 0x2000 centres it: x plus the truncated half of w minus 1 minus the width. 0x3000 puts the right edge inside: x plus w minus the width minus the unit. 0x4000 puts it outside: x plus w plus the unit. Any other word uses x plus the inset plus half of the unit rounded up.

The vertical style is the style word masked with 0x700. Exactly 0x100 moves up by half the unit. 0x200 centres with the line height: y plus the truncated half of h plus the line height plus 2. 0x300 sits at the base: y plus h plus the base minus 1. 0x400 sits at the bottom: y plus h. Any other word moves down by half the unit rounded up.

The state word chooses the ink. Bit 0x10 selects the first ink with no nudge. Mode 0x10000 selects the coloured renderer, with a one-pixel nudge when bit 0x40 is set, and no ink bind of its own. Bits 0x60 select the third ink with a one-pixel nudge. Any other word selects the second ink with no nudge. One text call follows.

The three ink fields are read from the skin in the observed order. The exact field names are not established.

## Open questions

The glyph renderer and the string storage are not in. The methods around this one are not in.

## Docs

This method is 254 bytes. With the span bar, the square, the chip and the capped span, that is 1,830 bytes of the 24 KB class, which is 7%. `TGSLines` is 7%. `TGUIDrawSkin` stays 49%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
