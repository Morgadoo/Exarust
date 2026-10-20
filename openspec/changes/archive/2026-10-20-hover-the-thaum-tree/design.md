# Design

## Behaviour

The event kind at its first word must be 0xae56ca01. The mouse point comes from the screen object's 0x18 and 0x1c words after two adjustments, and the node hit test returns a positive index when the point is over a node.

A positive index copies the entry at that index over the previous entry when the previous index is positive, sets the entry's flag word to its old 0x700000 bits OR the event's first word, stores the event's 0x20 word, plays the hover sound when its global is positive and returns true. A non-positive index copies three default words over the previous entry when it is positive and returns false.

## Open questions

The hit test, the adjustments and the sound are not in.

## Docs

This method is 384 bytes. `TThaumTree` moves from 21% to about 22%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
