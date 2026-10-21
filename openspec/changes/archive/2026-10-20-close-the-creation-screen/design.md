# Design

## Behaviour

When the child's active word is not 1, the stored byte clears and the global mode chooses the scheduled form: 10 is the special form, anything else the regular form.

With an active child, the first present of three held objects is released; the middle one also gets a call with `0x2d`. Then two more are released when present. A mode other than 12 releases two further objects (both with argument 1) and writes `0x110` to two widget fields.

The tick runs, the active word clears, two widget fields receive their strings, the widget loses bit `0x10`, a second widget receives its string, loses bit `0x10` and is called with `0x10`.

## Model

`close_creation_screen(input)` returns the scheduled form or none, the seven release flags, the `0x2d` call, the widget reset, the tick and the final string/disabling flags.

## Open questions

The release, tick and string bodies are not in.

## Docs

This method is 442 bytes. `SG_Character_Create` moves from 23% to about 25%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
