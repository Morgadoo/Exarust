# Design

## Behaviour

The detach returns true immediately when the pane is already inactive. Otherwise it marks the owner (setting bit 0x20000000 in its word) when one exists, and places the child when a child exists and its visibility byte is set. A stack pointer above one has its 0x18 word cleared and its count decremented; a stack of one is freed and the pointer cleared. When the pane is the loot pane its origin words copy to two globals. The base method runs once and its result returns.

The page override calls the base method, then stores −3 at field 0x6c when its argument is −1. Any other argument leaves the field.

## Open questions

The owner, child and stack objects are named only by offset. The base method is not in.

## Docs

The two methods are 259 and 41 bytes. `SG_GameGUI` is 5%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
