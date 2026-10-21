# Design

## Behaviour

The close clears `0x4ef330` and returns the special form for mode 10, else the regular form.

The key checks the selection; above zero it calls the base key handler with the key and the masked argument and returns its result; otherwise it returns `0x40` and calls the close slot with zero.

The show calls the input reset with the argument and one, the screen setup with the two globals, then the continue button update, and when the save-record conversion is true it builds the continue entry from the global's list.

The capture sets the stored byte, copies six qwords from the child's slot table, computes the child's index less one global plus another, stores the child's mode, clears the stored byte and schedules the next form.

## Model

`close_creation_form(mode)`, `creation_key(has_selection, base)`, `show_creation_form(build)` and `capture_creation(words, current, last, saved, mode)` return these values.

## Open questions

The called bodies are not in.

## Docs

These methods are 64, 65, 75 and 123 bytes. `SG_Character_Create` moves from 26% to about 27%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
