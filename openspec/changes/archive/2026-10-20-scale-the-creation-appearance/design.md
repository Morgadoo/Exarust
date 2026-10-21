# Design

## Behaviour

The byte scale reads three floats (two of the child's and one of the object's record), multiplies each by 255 and converts with `cvtss2si`, then stores the low bytes in three consecutive bytes. The word scale reads the first two floats, adds 0, multiplies by 65535 and stores the low 16 bits in two consecutive words.

The conversion rounds to the nearest even integer. Values above one wrap in the low byte or word exactly as the stores do.

## Model

`appearance_bytes([f32; 3]) -> [u8; 3]` and `appearance_words([f32; 2]) -> [u16; 2]`. The two calls that follow each scale in the original are out of scope.

## Open questions

The two following calls are not in.

## Docs

These methods are 173 and 149 bytes. `SG_Character_Create` moves from 6% to about 7%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
