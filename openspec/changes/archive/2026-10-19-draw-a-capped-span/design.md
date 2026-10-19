# Design

## Behaviour

A mode call with argument 2, one begin, one ink bind, one preparation, two cap sample binds, one middle sample bind, twelve corner writes and one end call follow.

With unit U, the doubled extent is 2U and the quadrupled extent is 4U. The top is the centre minus 2U and the bottom is 4U plus the top. The three quads are:

- left cap: (left - 2U, top), (left - 2U, bottom), (left + 4U, bottom), (left + 4U, top)
- right cap: (right + 2U, top), (right + 2U, bottom), (right - 4U, bottom), (right - 4U, top)
- middle: (left + 4U, centre), (left + 4U, bottom), (right - 4U, bottom), (right - 4U, centre)

The second positional float of the retail method is never read. The Rust function omits it and documents that the retail argument is dead.

The corners are placed with the shared view transform and twelve colour writes are counted. No mark is stored.

## Open questions

The caller that supplies the anchors is not in. The methods around this one are not in.

## Docs

This method is 495 bytes. With the span bar's 470, the square's 210, the chip's 401 and the bevel's 775, that is 2,351 bytes of the 24 KB class, which is 9%. `TGSLines` is 9%. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
