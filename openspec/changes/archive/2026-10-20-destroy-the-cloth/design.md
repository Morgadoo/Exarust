# Design

## Behaviour

The destructor makes one release call when the free flag is positive. It frees the pointers at 0xd8, 0xe0, 0xe8 and 0x128 when each is present. When the cloth is its own root it also frees the pointer at 0x120 when present. It destroys the managed fields at 0xf8, 0x190 and 0x138, then, when the list pointer at 0x198 is present, frees each of the body's 0x1c8 entries and then the list. The base destructor always runs, and a positive free flag makes one final call.

## Open questions

The release, field and base bodies are not in.

## Docs

This method is 416 bytes. `TVCCloth` is 78%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
