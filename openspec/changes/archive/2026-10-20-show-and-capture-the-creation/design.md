# Design

## Behaviour

The close clears `0x4ef330` and returns the special form for mode 10, else the regular form.

The key checks `0x6c`; above zero it calls `0x10002dbc0` with the key and the masked argument and returns its result; otherwise it returns `0x40` and calls slot `0xc8` with zero.

The show calls `0x100139430` with the argument and one, `0x1001316d0` with the two globals, then `0x10012f840`, and when `0x10017e740` is true it calls `0x10012f7e0` with the global's `0x98`.

The capture sets the stored byte, copies six qwords from `0x158 + 0xc0`, computes `child+0x258 − global 0x4f0430 + global 0x4f0b80`, stores the child's `0x274`, clears `0x4ef330` and schedules `0x1001246d0`.

## Model

`close_creation_form(mode)`, `creation_key(has_selection, base)`, `show_creation_form(build)` and `capture_creation(words, current, last, saved, mode)` return these values.

## Open questions

The called bodies are not in.

## Docs

These methods are 64, 65, 75 and 123 bytes. `SG_Character_Create` moves from 26% to about 27%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
