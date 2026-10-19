# Design

## Behaviour

One virtual prepare call, mode 1, one rectangle batch and mode 0x10 follow. The rectangle's left and top are the place's x and y plus the stored inset; its width and height subtract twice the inset. The four corners are (right, top), (left, top), (left, bottom), (right, bottom). The colour is the place's four bytes with alpha 255. Four colour writes and four vertices are counted.

## Open questions

The virtual prepare body is not in.

## Docs

This method is 141 bytes. `TGUIDrawSkin` stays 34%. `TGSLines` stays 87%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
