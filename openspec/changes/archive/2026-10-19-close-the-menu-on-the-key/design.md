# Design

## Behaviour

The override takes a 16-bit mask and a base handler. When mask & 0x2002 equals 0x2002 it closes the form with argument 0 and returns 1; the base handler does not run. Any other mask returns whatever the base handler returns and does not close. Both the journal and the character-loot class use this same body.

## Open questions

The base handler and the close's body are not in. The mask bits' names are not established.

## Docs

Each method is 89 bytes. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
