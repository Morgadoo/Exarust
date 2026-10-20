# Design

## Behaviour

The scan reads each slot's low word at `0xc0 + i × 8` and stops at the first non-zero one. That slot's word and bytes 3 through 7 are zeroed; byte 2 is left as it was. The next loop runs from the found index plus one through index 4 and zeroes those slots whole, eight bytes each. Slot 5 is reached only when it is itself the found slot, and then only its word and bytes 3 through 7 are cleared.

With no non-zero word the method returns without touching memory.

## Model

`clear_creation_slots(&mut [[u8; 8]; 6])` returns the found index, or `None`. The byte-2 preservation and the unchecked fifth slot are reproduced.

## Open questions

What the slots hold is not established.

## Docs

This method is 134 bytes. `SG_Character_Create` stays about 2% (about 2.2% by bytes). The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
