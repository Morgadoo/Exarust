# Design

## Behaviour

The base method always runs. When the object pointer is absent nothing else happens. Otherwise the object's 0x1c word loses bits 0x60. When the selection is above zero it is applied as-is. When it is not, and the object word's low 16 bits are nine or ten, −1 is applied. Any other word applies nothing.

## Open questions

The apply call's body is not in. The word's source is named only by offset.

## Docs

This method is 141 bytes. `SG_GameGUI` stays 8%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
