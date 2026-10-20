# Design

## Behaviour

The scale is the source at `0x38`, converted to double, multiplied by the double `0.17`, added to the double `0.9` and converted back to a float. It is stored at the child's `0x1a0` and at `0x158 + 0x50`.

With bit `0x100000` set on the object's `0x1c`, a clear byte at `0x1002b3d60` runs `0x1000cf330`. When the child's label at `0x198` is null, one is built from `0x1b0` and `0x190 + 1` and stored. The label is then drawn through `0x1000cd390` with the scale.

## Model

`apply_creation_scale(source, special, global_flag, has_widget)` returns the scale, the store flag, the rebuild flag, the global call, the build and the draw. The finalizer always runs.

## Open questions

The global call, build and draw bodies are not in.

## Docs

This method is 307 bytes. `SG_Character_Create` moves from 21% to about 22%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
