# Design

## Behaviour

The stored charge at 0x16c becomes the double sum of the current value and the tick over 120, converted to single precision and clamped to −9 through 5. The gain at 0x194 is `(5 + charge) × 0.2` when the charge is non-negative, else `(4 − √−charge) × 0.25`. A clear started byte is set and the clock stored at 0x1a0; later calls leave both.

## Open questions

The sound call with argument 0x23 is not in.

## Docs

This method is 316 bytes. `TThaumTree` moves from 18% to about 19%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
