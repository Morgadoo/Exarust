# Design

## Behaviour

When no row is found nothing shows and all outputs stay empty. When a row is found, the index is the object's 0xb8 word less one and the threshold is the row's 0x38 word less one. The flags gain 0x10 and the visible byte is set. The place is the anchor's x and nine units plus the anchor's y; the height is 24 units when global 0x2ad3c0 is zero and 18 otherwise. Three calls run. The stamp is three times the base global plus the pane's 0x120 offset.

## Open questions

The row marker 0xac3 and the anchor adjustment call are not in. The three calls' bodies are not in.

## Docs

This method is 372 bytes. `SG_GameGUI` is 8%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
