# Design

## Behaviour

The method takes four floats. Let a be the first, b the second, c the third and d the fourth. The middle y is (b − d) × 0.5 + d. One begin, four binds and eight vertices follow:

1. (c, b) with slot 0x12c
2. (a, b) with slot 0x12c
3. (c, mid) with slot 0x134
4. (a, mid) with slot 0x134
5. (c, mid) with slot 0x138
6. (a, mid) with slot 0x138
7. (a, d) with slot 0x130
8. (c, d) with slot 0x130

One end closes the strip. No colour is written.

## Open questions

The caller's four floats and the texture contents are not established.

## Docs

This method is 339 bytes. `TGUIDrawSkin` is 45%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
