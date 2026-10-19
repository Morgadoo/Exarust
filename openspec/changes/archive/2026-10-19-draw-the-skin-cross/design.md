# Design

## Behaviour

One begin, eight vertex appends and one line end follow. With unit U and half extent H = ((U + 1) >> 1) + 1, the corners are:

1. (x - H + 2, y - U)
2. (x + H - 1, y - U)
3. (x - H + 2, y + U)
4. (x + H - 1, y + U)
5. (x - U + 1, y - H + 2)
6. (x - U + 1, y + H - 1)
7. (x + U + 1, y - H + 2)
8. (x + U + 1, y + H - 1)

The eight vertices are four line segments. No colour is bound.

## Open questions

The caller that picks the point is not in.

## Docs

This method is 385 bytes. `TGUIDrawSkin` is 35%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
