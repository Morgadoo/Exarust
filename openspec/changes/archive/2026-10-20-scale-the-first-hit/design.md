# Design

## Behaviour

The method returns unchanged when the combatant's body flags carry 0xa00 or its helpless byte is set. Otherwise the first hit scales the current value by `(1 + stat) × 0.025 + 0.95`, both in double precision. When the target's flag word lacks 0x100000 the target's hit rises to the combatant's threshold if it is lower, and the bit is set. The body's own 0x100000 bit makes one special call.

## Open questions

The special call's body and the hit's physical resolution are not in.

## Docs

This method is 238 bytes. The combat-rule figure moves from 27.3% to about 30%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
