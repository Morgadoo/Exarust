# Design

## Behaviour

The raw word is reduced to its low 16 bits. An index above 8 becomes 0. When the page field at 0x160 is 1, a positive index sets bits 0x110 on the object flags and a zero index clears them. Any other page leaves the flags untouched.

## Open questions

The page field and the flag bits' names are not established. The base handler is not in.

## Docs

This method is 108 bytes. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
