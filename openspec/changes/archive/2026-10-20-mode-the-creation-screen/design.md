# Design

## Behaviour

Switching with mode 2 stores the object's `0x158` at the global `0x1004f0890` and calls `0x100111670`. Any other mode looks up the widget at `0x4534`; a positive mode clears bit `0x10` of its `0x1c`, a zero or negative mode sets it.

Toggling reads the mode at `0x274`. From mode 2 it needs flag bit 1, sets mode 1 and refreshes. From any other mode it needs flag bit 2, sets mode 2 and refreshes. Without the bit nothing changes. The refresh is `0x10012f950` with the object's `0x254`.

## Model

`switch_creation_mode(mode)` returns either the special branch or the widget's enabled flag. `toggle_creation_mode(mode, flags)` returns whether the mode changed, the new mode and whether the refresh ran.

## Open questions

The special call, widget lookup and refresh are not in.

## Docs

These methods are 74 and 111 bytes. `SG_Character_Create` stays about 23% (about 23.3% by bytes). The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
