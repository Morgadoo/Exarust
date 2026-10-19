# Design

## Behaviour

The method reads a selection at 0x168, a page at 0x160, a word at 0x6c and a mode byte at 0x16c. When the selection is above zero and the page is above one, the base handler runs and its result returns. Otherwise, when the word's low 16 bits equal nine and the mode byte is zero, the base handler also runs. In every other case the form closes with argument 0 and the method returns 0x40.

## Open questions

The page, word and mode fields are named only by offset. The base handler is not in.

## Docs

This method is 131 bytes. The UI section stays 0%. `TGUIDrawSkin` stays 55%. `TGSLines` stays 87%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
