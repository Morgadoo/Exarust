# Design

## Behaviour

The destructor makes one release call when the free flag is above zero, always runs the base destructor once, and makes one final call when the free flag is set.

The clear checks for a view pointer. When present it makes one release call on it, writes zero to the owner's 0x130 word and clears the pointer. The base method always runs and its result returns.

## Open questions

The release and base bodies are not in. The owner is named only by offset.

## Docs

The two methods are 73 and 86 bytes. `SG_GameGUI` stays 8%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
