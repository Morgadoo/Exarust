# Design

## Behaviour

Release runs the base once and clears bit 0 of the 0x14 word of each of the counted entries at 0x160, stride 0x18.

Scale runs the base once and, for each entry, divides its x and y by the argument as doubles, multiplies by the unit and rounds to nearest even. A zero argument is not guarded, as in the original.

## Open questions

The base methods are not in.

## Docs

The two methods are 104 and 141 bytes. `TGUIItemContainer` moves from 26% to about 27%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
