# Design

## Behaviour

The loot screen checks its inactive byte. When set it returns true without any other work. Otherwise it sets bit 0x20000000 in the owner's 0x124 word, places the child when the child exists and its 0x21b visibility byte is set, runs the base method once and returns its result.

The journal copies the form's x and y to two globals, reads the object at 0x160, assigns the string at the global table plus the page times 0x18 plus 0x10, then runs the base method once.

## Open questions

The owner, child and table objects are named only by offset. The base methods are not in.

## Docs

The two methods are 112 and 93 bytes. `SG_GameGUI` stays 8%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
