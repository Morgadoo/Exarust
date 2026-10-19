# Design

## Behaviour

The method asks the form for its hover value. When that value equals field 0x204 the candidate is field 0x204, else zero. When the candidate equals field 0x70 the method returns 0. Otherwise it stores the candidate in field 0x70 and returns 3. If the last sound stamp plus 0xc8 (200) is strictly below the clock, it plays a sound when the candidate is non-zero and stores the clock in the stamp; otherwise it plays nothing and leaves the stamp.

## Open questions

The hover virtual and the sound call are not in. The clock's unit is not established.

## Docs

This method is 199 bytes. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
