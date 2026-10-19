# Design

## Behaviour

The measured text is aligned exactly as the line label: the horizontal style is 0x7000 and the vertical style 0x700, with the same five arms each.

The state word then chooses the calls. Bit 0x10 draws once in the first ink. Mode 0x10000 draws once through the coloured renderer, one pixel lower when bit 0x40 is set, with no ink bind. Otherwise bit 0x40 draws twice: one pixel left in the second ink, then one pixel down in the third ink. Any other word draws once in the fourth ink.

The four ink fields are read from the skin in the observed order. Their names are not established.

## Open questions

The glyph renderer and the string storage are not in.

## Docs

This method is 305 bytes. `TGUIDrawSkin` is 34%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
