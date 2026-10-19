# Design

## Behaviour

When mask & 0x2002 equals 0x2002 the method closes with argument 0 and returns 1. If the callback pointer at 0x188 is non-null it calls it with the argument at 0x190 and 0; otherwise it calls the form's close. Any other mask runs the base handler and returns its result.

## Open questions

The callback's owner and the form close are not in. The mask bits' names are not established.

## Docs

This method is 116 bytes. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
