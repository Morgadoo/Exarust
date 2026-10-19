# Design

## Behaviour

A mode call with argument 2, one begin, one ink bind, one preparation, four sample binds, sixteen corner writes and one end call follow.

The bottom of the track is the place's y plus its height minus three units. The right edge is the place's x plus its width. The track quads are:

- left cap: (x - 2U, bottom), (x - 2U, bottom + 4U), (x + 4U, bottom + 4U), (x + 4U, bottom)
- right cap: (right - 2U, bottom), (right - 2U, bottom + 4U), (right - 4U, bottom + 4U), (right - 4U, bottom)
- middle: (x + 4U, bottom + 2U), (x + 4U, bottom + 4U), (right - 4U, bottom + 4U), (right - 4U, bottom + 2U)

The thumb is (right - U, y + 2U), (right - U, y + 4U), (right - 3U, y + 4U), (right - 3U, y + 2U).

Bit 0x10 of the state selects the first ink and any other word the second. Mode 0x10000 rebinds the third ink with alpha 255 before the thumb; otherwise the thumb keeps the track ink. The corners are placed with the shared view transform.

## Open questions

The value that positions the thumb is not in; this slice fixes it at the right edge. The methods around this one are not in.

## Docs

This method is 873 bytes. With the span bar, the square, the chip, the capped span and the label, that is 2,703 bytes of the 24 KB class, which is 11%. `TGSLines` is 11%. `TGUIDrawSkin` stays 49%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
