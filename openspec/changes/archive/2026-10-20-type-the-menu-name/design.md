# Design

## Behaviour

When key bit 0x2 is clear the base handler runs and the buffer is unchanged. Otherwise the buffer's nine tail bytes move one place left, the last byte becomes the character, and characters from A through Z gain 0x20. The buffer is then compared with the ten bytes "barechoons"; equality runs the submit routine.

## Open questions

The base handler and the submit routine are not in. The name entry's origin field is not needed.

## Docs

This method is 225 bytes. `SG_GameMain` is 10%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
