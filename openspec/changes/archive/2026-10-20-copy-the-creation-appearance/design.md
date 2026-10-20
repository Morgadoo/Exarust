# Design

## Behaviour

The distance between the source and the target is compared with the same epsilon as the settle. Under it nothing is written. Otherwise the target takes the source and the redraw debounce runs: a last stamp plus 2 above the clock sets the pending byte, and otherwise the stamp takes the clock, the pending byte clears and the callback at slot `0x1b8` runs.

Both routines share the debounce; only the computed value differs. The settle transforms the source, the copy stores it raw.

## Model

`copy_appearance(source, target, last, clock)` returns the same `CreationSettle` shape as `settle_appearance`. A private `debounce` holds the shared branch.

## Open questions

The callback is not in.

## Docs

This method is 135 bytes. `SG_Character_Create` stays about 3% (about 3.4% by bytes). The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
