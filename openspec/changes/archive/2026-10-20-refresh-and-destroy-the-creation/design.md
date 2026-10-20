# Design

## Behaviour

The part refresh reads the object's `0x160`, follows its `0x2e0` and `0x18`, runs the part search and, when the target at `0x180` exists, stores the result at the target's `0x180`.

Destroying calls slot `0x90` when the flags are positive. A child at `0x260` with a count above one clears its `0x18` and loses one from the count; at one or below it is released. The base `0x10002aae0` always runs, and slot `0x70` frees when the object and flags are present.

## Model

`refresh_creation_part(has_target, part_found)` returns the store and part flags. `destroy_creation(flags, has_child, child_count)` returns the base destroy, decrement, release, base and free flags.

## Open questions

The search, release and free bodies are not in.

## Docs

These methods are 64 and 137 bytes. `SG_Character_Create` moves from 30% to about 32%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
