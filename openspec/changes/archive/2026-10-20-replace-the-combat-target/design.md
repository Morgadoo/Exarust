# Design

## Behaviour

The list is scanned for the first entry equal to the old target and that entry's key becomes the new target. Slots 0x778 and 0x770 become the new target when they equal the old. The body's hit pointer at 0xe38, whose stored target is at 0xd40, becomes the new target's 0x10 word when it equals the old, and its contact pointer at 0xe50 becomes the new target when it equals the old. Anything that does not match is left alone.

## Open questions

The target objects and the entry's other fields are not needed.

## Docs

This method is 180 bytes. The combat-rule figure stays about 29%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
