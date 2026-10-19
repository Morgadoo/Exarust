# Design

## Behaviour

The mask's bits are tested in order. Bit 0x100 with a positive selection plays the sound and calls the confirm virtual with the selection and the caller's argument; a zero selection does nothing. Bit 0x200 calls the forward helper and bit 0x400 the back helper. With bits 0..1 set, direction 0x8000 moves to one more than the selection, at least 1, when the selection is below the limit; direction 0x4000 moves to one less when the selection is above 1. Any other direction does nothing. The result is the called helper's result, or zero when nothing ran.

## Open questions

The confirm, forward and back helpers are not in; the caller's sub-result stands in for them.

## Docs

This method is 328 bytes. `SG_GameMain` is 9%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
