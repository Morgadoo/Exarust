# Design

## Behaviour

Bounds ignores its content and writes the screen to the first output, −1 to the second, and returns −1.

Destroy runs in a fixed order. A set free flag makes one release call. The splash global is cleared. Each of the two stored pointers is freed when present. A list count above zero makes one list call. The base destructor always runs. A set free flag makes one final call; a clear flag makes none. The input self is always non-null on this path.

## Open questions

The release, list and base bodies are not in. The pointers' types are named only by offset.

## Docs

The two methods are 28 and 157 bytes. `SG_GameMain` stays 10%. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
