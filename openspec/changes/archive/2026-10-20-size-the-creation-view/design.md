# Design

## Behaviour

The link copy reads the word at the object's `0x170 + 0x158` into `0x158 + 0x2c`. The debounce then compares the stamp at `0x1e0` plus two with the clock: at or below the clock the stamp takes the clock, the pending byte `0x1e4` clears and the callback at slot `0x1b8` runs; above it the pending byte sets.

Attaching stores the body at `0x160`, its `0x2e0` at `0x168` and its `0x2d0` at `0x158`. A pane at `0x180` receives the body's `0x1f0` at its `0x168`. A link at `0x170` receives the renderer's `0x30` at its `0x160`.

Sizing reads the frame at `0x158 + 0x40`, adds one, or uses one when there is no frame, and calls `0x100040180` on `0x188`. The unit at `0x1002ad3f0` times 12 is stored at the child's `0x180 + 0x4c` and at `0x170`. A present global at `0x4ef320` runs the tick `0x10012c5f0`.

## Model

`copy_creation_link(stamp, clock)`, `attach_creation_body(has_pane, has_link)` and `resize_creation_view(frame, unit, has_global)` return the flags and values above.

## Open questions

The callback, pane, link and tick bodies are not in.

## Docs

These methods are 102, 113 and 117 bytes. `SG_Character_Create` moves from 19% to about 20%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
