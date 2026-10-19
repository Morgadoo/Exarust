# Design

## Behaviour

The outer bar halves the unit with a logical integer shift, not a float division. With x, y, w, h and a side inset s: the right edge is x plus w, the top is y minus half, the bottom is y plus h plus half, the left inset edge is x plus s and the right inset edge is the right edge minus s.

The six quads are:

- cap: (L, T+), (L, T-), (x, T-), (x, T+)
- bar: (L, T+), (L, T-), (Q, T-), (Q, T+)
- cap: (Q, T+), (Q, T-), (R, T-), (R, T+)
- cap: (L, B-), (L, B+), (x, B+), (x, B-)
- bar: (L, B-), (L, B+), (Q, B+), (Q, B-)
- cap: (Q, B-), (Q, B+), (R, B+), (R, B-)

where L = x + s, Q = R - s, T is the top, B is the bottom, T+ and T- add and subtract the unit, and B+ and B- do the same.

The binds are cap, bar, cap, cap, bar, cap. The outer ink is the special ink with alpha 255 when the state band is 0x10000, else the normal ink. The inner ink copies four bytes from the place and forces alpha 255.

The inner rectangle writes the unit back into the place as a 16-bit value and runs from x plus unit, y plus unit to x plus w minus unit, y plus h minus unit. Its corners are (right, top), (left, top), (left, bottom), (right, bottom).

Three mode calls (2, 1, 0x10), one preparation, two begins, two ends, twenty-eight vertices and twenty-eight colour writes are counted.

## Open questions

The owner's side inset is not in. The methods around this one are not in.

## Docs

These two methods are 1,104 and 201 bytes. With the frame, the span bar, the square, the chip, the capped span, the label, the rail, the heights and the hit test, that is 5,924 bytes of the 24 KB class, which is 24%. `TGSLines` is 24%. `TGUIDrawSkin` stays 49%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
