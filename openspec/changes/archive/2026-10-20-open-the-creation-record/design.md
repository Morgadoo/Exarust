# Design

## Behaviour

The reset clears five consecutive words of the record, then its tail, and calls the record reset with the size 8 and a zero argument. The slot table writes three eight-byte entries: the low word is `0x61a8`, then the bytes 1, 2, 3 and 4, then 8, 5 and 6 respectively in byte 6. Byte 7 is not written and is zero in a fresh record.

## Model

`reset_creation_record()` returns the five zero words, the zero tail and the reset size. `creation_slot_defaults()` returns the three slot bytes.

## Open questions

The reset body is not in.

## Docs

These methods are 84 and 99 bytes. `SG_Character_Create` moves from 17% to about 18%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
