# Design

## Behaviour

When the still bit of the flag word is set, the close mark selects one tint:

- -9: (x0+U, y0+2U), (x0+U, y0), (x0+3U, y0), (x0+3U, y0+2U)
- -10: (x0+U, y0-2U), (x0+U, y0), (x0+3U, y0), (x0+3U, y0-2U)
- -8: (x0, y0+4U), (x0, y0), (x0+4U, y0), (x0+4U, y0+4U)

x0 is the close x plus 0.15 times the unit rounded to nearest even. y0 is the close y plus three units when mask bits 1 and 4 are both set, else two units. The quad binds the tab sample and the colour 255, 204, 153, 255. Any other mark draws nothing.

## Open questions

The click path that produces the marks is not in. The other dispatcher branches are unchanged.

## Docs

This closes three branches of the already-counted dispatcher. `TGSLines` stays 87%. `TGUIDrawSkin` stays 32%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
