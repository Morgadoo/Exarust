# Design

## Behaviour

The method stores its index at field 0x6c. When the index is above zero and the sound flag is above zero it plays one selection sound. It always returns 3. The sound call's fixed arguments are a zero dword, a pointer and two floats; they are not exposed because the call is reported as one played sound.

## Open questions

The sound object and the fixed arguments are not named. The caller's use of the return 3 is not in.

## Docs

This method is 88 bytes. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
