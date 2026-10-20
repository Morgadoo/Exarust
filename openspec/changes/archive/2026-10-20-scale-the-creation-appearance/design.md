# Design

## Behaviour

The byte scale reads three floats at `0x158`, `0x15c` and the object's `0x1a8 + 0x38`, multiplies each by 255 and converts with `cvtss2si`, then stores the low byte at `0x44`, `0x45` and `0x46`. The word scale reads two floats at `0x158` and `0x15c`, adds 0, multiplies by 65535 and stores the low 16 bits at `0x3c` and `0x3e`.

The conversion rounds to the nearest even integer. Values above one wrap in the low byte or word exactly as the stores do.

## Model

`appearance_bytes([f32; 3]) -> [u8; 3]` and `appearance_words([f32; 2]) -> [u16; 2]`. The two calls that follow each scale in the original are out of scope.

## Open questions

The two following calls are not in.

## Docs

These methods are 173 and 149 bytes. `SG_Character_Create` moves from 6% to about 7%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
