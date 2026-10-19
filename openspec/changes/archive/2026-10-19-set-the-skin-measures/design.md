# Design

## Behaviour

The method writes four 32-bit words into the skin at offsets 8, 0xc, 0x10 and 0x14. The first three come from the globals at 0x2ad470, 0x2ad460 and 0x2ad400. The fourth is the unit times four plus four, wrapping. It returns nothing.

## Open questions

The three globals' names are not established. The unit initialiser shows the third is six units less the trim.

## Docs

This method is 48 bytes. `TGUIDrawSkin` stays 47%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
