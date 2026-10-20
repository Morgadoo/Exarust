# Design

## Behaviour

The copy moves twelve bytes from the child's `0x158 + 0xc` to the object's `0x10`.

The resume calls `0x10017f500` on the global list at `0x4f0c20`, then `0x10012f950` with the child's `0xc0` and its `0x254`, and finally slot `0xc8` with the child and the object.

## Model

`copy_creation_point([i32; 3])` returns the same three words. `resume_creation()` returns the save, refresh and resume flags.

## Open questions

The save, refresh and slot bodies are not in.

## Docs

These methods are 39 and 71 bytes. `SG_Character_Create` stays about 32% (about 32.5% by bytes). The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
