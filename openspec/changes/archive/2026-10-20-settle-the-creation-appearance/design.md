# Design

## Behaviour

The value is `(0.5 − source) × 2`. Its distance from the target is compared with the epsilon `0x33ffffd7`, about `2e−7`; under it nothing is written and the stamp and pending flag stay. A NaN distance also stores nothing.

Otherwise the pending byte is either set or cleared. When the last stamp plus 2 is greater than the clock, only the pending byte is set. When it is not, the stamp takes the clock, the pending byte is cleared and the callback at slot `0x1b8` runs with the object at `0x1c0`. An unordered distance from a NaN source is not under the epsilon, so it stores like any other distance.

## Model

`settle_appearance(source, target, last, clock)` returns the stored value if any, the pending flag, the stamp and whether the callback ran. The epsilon is built from its exact bit pattern.

## Open questions

The callback is not in.

## Docs

This method is 154 bytes. `SG_Character_Create` moves from 2% to about 3% (about 2.8% by bytes). The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
