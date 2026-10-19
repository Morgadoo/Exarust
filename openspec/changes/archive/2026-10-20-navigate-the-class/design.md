# Design

## Behaviour

The available mask is `(available << 1) | 1`, so bit zero is always set and class n is bit n. Mask 0x100 with a positive selection returns 7 and shows form 0x10012a870 after the same globals and sound as the confirm; a zero selection returns 0. Masks 0x200 and 0x400 call 0x10012ac60 and return 0.

With mask bits 0..1 set, direction 0x2000 clamps the selection to zero, then scans from that index to 3 for the first set bit and targets that index plus one; above 3 it does not move. Direction 0x1000 targets 1 when the selection is at most 2; otherwise it scans down from selection − 2 to zero for the first set bit and targets that index plus one. When the target differs from the selection the selection virtual runs with the target.

## Open questions

The forward call and the selection virtual are not in.

## Docs

This method is 437 bytes. `SG_GameMain` is 13%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
