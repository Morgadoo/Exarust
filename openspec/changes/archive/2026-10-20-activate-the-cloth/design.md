# Design

## Behaviour

Activation returns the four flag words with their new bits: 0x300 gains 0x20000000, 8 gains 0x10000, 0xc gains 0x204030 and 0x188 gains 0x6000. It sets the phase to 2 and the field at 0x180 to 0.4. A present handle writes once, a root object copies its origin once, and a set slow global adds 0x8000 to the object's 0xc word.

The transform runs three calls for each particle index, covering the three buffers. It makes one call on the object's 0x10 field when the object flags carry 0x20, and always one on its 0x20 field.

## Open questions

The transform calls' bodies and the base call are not in.

## Docs

The two methods are 228 and 176 bytes. `TVCCloth` is 72%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
