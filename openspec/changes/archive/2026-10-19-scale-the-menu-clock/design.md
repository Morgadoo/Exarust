# Design

## Behaviour

The method takes one 32-bit clock word. It converts it to a double, multiplies by 0.007407407407407408 (1/135), converts to an integer with round-to-nearest-even, converts that back to a double, multiplies by 0.125 (1/8), converts to single precision and stores it at field 0x158.

## Open questions

The source clock, a field of a global object, is passed in; its name is not established. The base update is not in.

## Docs

This method is 72 bytes. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
