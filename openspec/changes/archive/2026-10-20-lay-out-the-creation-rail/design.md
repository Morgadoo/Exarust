# Design

## Behaviour

The count is stored on the object. The unit is a global. The child's view width becomes `width − before − unit × count`, where width is the object's and before the child's leading space.

When the global at `0x4effa0` is not 12, the widget at `0x98` receives a thumb of `(count − 4) × unit` at `0x140` and a start of `width − (count − 2) × unit` at `0x13c`. Both are stored as 16-bit words. In mode 12 neither is written.

## Model

`lay_out_creation_rail(count, unit, width, before, compact)` returns the view and, unless compact, the two 16-bit values.

## Open questions

The refresh call is not in.

## Docs

This method is 156 bytes. `SG_Character_Create` moves from 10% to about 11%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
