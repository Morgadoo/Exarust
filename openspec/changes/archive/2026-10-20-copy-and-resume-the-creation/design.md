# Design

## Behaviour

The copy moves twelve bytes from the child's record into the object.

The resume calls the save routine on the global list, then the refresh with two of the child's fields, and finally a slot callback with the child and the object.

## Model

`copy_creation_point([i32; 3])` returns the same three words. `resume_creation()` returns the save, refresh and resume flags.

## Open questions

The save, refresh and slot bodies are not in.

## Docs

These methods are 39 and 71 bytes. `SG_Character_Create` stays about 32% (about 32.5% by bytes). The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
