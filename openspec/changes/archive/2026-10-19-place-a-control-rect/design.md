# Design

## Behaviour

The metrics come from the unit U and a trim T: the close width is 4U + 1, the rail square is 4U − 1, the close height base is 6U − T, and the tab shift is U / 2 plus the close width. The unit initialiser writes exactly those four globals.

Seven codes are handled:

- −4 returns left = x − close − U, top = U − 1, right = left + close, bottom = base − U + 3.
- −6 shifts left by the tab shift when flag bit 1 is set.
- −5 shifts left by twice the tab shift when flag bits 1 and 3 are both set, once when only one is, and not at all when neither.
- −9 returns left = x − size − rail, top = field a, and a rail square.
- −10 returns left = x − size − rail, top = y − size − rail, and a rail square.
- −13 returns left = size, top = y − size − rail, and a rail square.
- −14 returns left = field b − rail, top = y − size − rail, and a rail square.

Any other code writes nothing. Every write is 32-bit wrapping arithmetic.

## Open questions

The control fields at 0x28, 0x2c, 0x30, 0x40 and 0x44 are named only by offset. The hit test that returns the codes is not in.

## Docs

This method is 807 bytes. `TGUIDrawSkin` is 37%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
