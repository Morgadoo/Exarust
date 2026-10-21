# Design

## Behaviour

The destination takes the four qwords at `0x64`, `0x6c`, `0x74` and `0x7c`, the count at `0x84` and the list at `0x88`; the source's list is then reset to empty and its count to 0. The same happens for the tail at `0xf0` and its list at `0xf8`. The source's four words are not cleared.

The dynamic-array copy and reset helpers move each list, so the ownership passes to the destination.

## Model

`CharacterAppearance` carries the words, count, parts, tail and tail parts. `move_appearance` returns the destination and leaves the source with zeroed counts and empty lists.

## Open questions

Nothing in the method is left out.

## Docs

This method is 228 bytes. `SG_Character_Create` moves from 1% to about 2%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
