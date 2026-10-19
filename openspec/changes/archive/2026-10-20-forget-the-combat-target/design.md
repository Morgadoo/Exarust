# Design

## Behaviour

The method scans the counted entries. On the first matching pointer it decrements the count and moves the entry at the new count over the found index, which is a no-op when the match was last. A missing target leaves the list untouched. The slots at 0x778 and 0x770 become zero when they equal the target. The body's hit pointer at 0xe38, whose stored target is at 0xd40, and its contact pointer at 0xe50 become zero when they equal it.

## Open questions

The entry's other fields are not needed beyond the 0x68-byte move.

## Docs

This method is 234 bytes. The combat-rule figure stays about 29%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
