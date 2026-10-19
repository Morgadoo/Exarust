# Design

## Behaviour

The confirm returns 0x40 when the selection is at most zero, after copying field 0x70 to field 0x204. With a positive selection, direction 0x1000 stores the selection at 0x70, copies it to 0x204, plays a sound when ready and returns 7; any other direction returns 0x40 without copying.

The select stores the index at 0x6c and returns 3. It plays a sound and stamps the clock only when the index is positive, the sound is ready and the previous stamp plus 200 is below the clock.

## Open questions

The sound call is not in.

## Docs

The two methods are 152 and 153 bytes. `SG_GameMain` is 16%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
