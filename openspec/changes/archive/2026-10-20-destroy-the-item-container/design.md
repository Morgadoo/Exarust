# Design

## Behaviour

The destructor makes one release call when the free flag is positive. It then walks the counted entries and calls 0x107a90 on each when the detached byte is set, else 0x1079e0. It destroys the array at 0x160, runs the base destructor once and makes one final call when the free flag is set.

## Open questions

The release and base bodies are not in.

## Docs

This method is 319 bytes. `TGUIItemContainer` moves from 27% to about 28%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
