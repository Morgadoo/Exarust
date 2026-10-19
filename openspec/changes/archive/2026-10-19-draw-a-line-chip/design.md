# Design

## Behaviour

A mode call with argument 2, one begin, one preparation, one sample bind, four colour writes, and one end call follow.

The chip starts at the place's x and y, sign-extended from 16 bits. The side is four times the unit. The quad is (x, y+S), (x, y), (x+S, y), (x+S, y+S), converted to floats.

The first sample is bound before the corners. When bit 0x40 of the flag word is set, the second sample is bound and the four corners are repeated. The sample arrays are the eight words 0.75, 0.8, 0.75, 1, 0.85, 1, 0.85, 0.8 and 0.85, 0.8, 0.85, 1, 0.95, 1, 0.95, 0.8.

The ink is the first colour when bit 0x10 is set. When the high part of the flag word masked with 0xf0000 is 0x10000, the ink is the second colour with its alpha forced to 255. Any other flag word uses the third colour. The other ink methods are not in.

## Open questions

The neighbouring line-class methods are not in. The text bind that the same method selects before the chip is not in.

## Docs

This method is 401 bytes. With the span bar's 470 and the square's 210, that is 1,081 bytes of the 24 KB class, which is 4%. `TGSLines` is 4%. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
