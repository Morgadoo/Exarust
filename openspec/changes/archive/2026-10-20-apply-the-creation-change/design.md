# Design

## Behaviour

When bit `0x40` of the object's `0x1c` word is clear, the method jumps to the finalizer and nothing else runs. Otherwise the child's `0x158` record gains bit 0 when the option word at `0x2` equals 2, and loses it otherwise. The appearance label is then taken through `0x10012d7a0`.

With the label known, the method sets the child's text, calls `0x10012d650` when the object at `0x1d0` is present, calls `0x1000cf330` when the global byte at `0x1002b3d60` is clear, sets the `0x1e5` flag, runs `0x10012cc50` and `0x10012cdf0`, clears the flag, runs the callback at slot `0x1b8`, and finally runs `0x10012d850`.

## Model

`apply_creation_change` returns the updated word, the label and a boolean for each call. The word keeps its other bits. The finalizer always runs, including on a disabled change.

## Open questions

What the two passes and the finalizer do is not established.

## Docs

This method is 218 bytes. `SG_Character_Create` moves from 3% to about 4%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
