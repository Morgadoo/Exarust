# Design

## Behaviour

When the selection is at most zero the method returns 0x40 and does nothing else. Otherwise it stores the selection, one less in the global class index, sets the result to 7, plays one sound when the sound global is positive, copies the page global into the mode global, prepares the character record with zero and shows the class form with flag zero.

## Open questions

The sound, the call and the form are not in. The globals are named only by address.

## Docs

This method is 154 bytes. `SG_GameMain` is 11%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
