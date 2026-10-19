# Design

## Behaviour

The method takes the two stored alpha bytes and the two scales. The first byte times the horizontal scale rounds to nearest even and keeps the low byte; the result fills 14 entries. The second byte times the vertical scale fills 48 entries the same way.

## Open questions

The ramps that read those bytes are not in. The methods around this one are not in.

## Docs

This method is 365 bytes. `TGUIDrawSkin` is 33%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
