# Design

## Behaviour

The span bar still builds the three raw quads. Each corner is then placed. The placed x is the raw x plus the x shift, times the x scale. The placed y is the y shift minus the raw y, times the y scale. The colour word is stored with every corner, twelve times.

The image does not store the shifts or the scales, so a fresh read is 0. The colour word in the image is 0. Those five values are arguments, because the writer reads them live. An unset view therefore places the first corner, raw (15, 20), at positive zero and negative zero: the y term is (0 − 20) times 0.

Shifts 2 and 100 with scales one half and 2 place that corner at 8.5, 160 and store colour 17.

## Open questions

When a flag bit 4 is set, the writer also stores a second word in a parallel list. That store is not in this slice.

## Docs

The writer is shared, so its bytes are not added to the line class. `TGSLines` stays 1%. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
