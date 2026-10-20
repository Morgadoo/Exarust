# Design

## Behaviour

When the child's `0x270` is not 1, the stored byte at `0x4ef330` clears and the mode at `0x4effa0` chooses the scheduled form: 10 is the special form `0x10018d400`, anything else the regular form `0x100127950`.

With an active child, the first present of `0x208`, `0x220` and `0x218` is released; the middle one also calls `0x10012e980` with `0x2d`. Then `0x238` and `0x210` are released when present. A mode other than 12 releases `0x228` and `0x1d0` (both with argument 1) and writes `0x110` to the widget's `0x28c` and `0x2f4`.

The tick `0x10002e760` runs, `0x270` clears, the widget's `0x68` and `0x138` receive their strings, its `0xec` loses bit `0x10`, the widget at `0x4534` receives its string, loses bit `0x10` and is called with `0x10`.

## Model

`close_creation_screen(input)` returns the scheduled form or none, the seven release flags, the `0x2d` call, the widget reset, the tick and the final string/disabling flags.

## Open questions

The release, tick and string bodies are not in.

## Docs

This method is 442 bytes. `SG_Character_Create` moves from 23% to about 25%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
