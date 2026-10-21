# Design

## Behaviour

Switching with mode 2 stores the object's record in a global and calls the record's handler. Any other mode looks up the mode widget; a positive mode clears bit `0x10` of its flag word, a zero or negative mode sets it.

Toggling reads the mode. From mode 2 it needs flag bit 1, sets mode 1 and refreshes. From any other mode it needs flag bit 2, sets mode 2 and refreshes. Without the bit nothing changes. The refresh takes the object's index.

## Model

`switch_creation_mode(mode)` returns either the special branch or the widget's enabled flag. `toggle_creation_mode(mode, flags)` returns whether the mode changed, the new mode and whether the refresh ran.

## Open questions

The special call, widget lookup and refresh are not in.

## Docs

These methods are 74 and 111 bytes. `SG_Character_Create` stays about 23% (about 23.3% by bytes). The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
