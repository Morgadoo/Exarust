# Design

## Behaviour

Showing writes zero to the stored pointer and byte, the seed to `0x1004f1b10`, and each list's child count less one to `0x1004f1b20`, `0x1004f1b30` and `0x1004f1b40`. Five panels are cleared through `0x100107b30` only when their count is positive. A non-null pointer at `0x728` is cleared through `0x100070e20`. The base `0x10002c410` returns the result, and a non-null global at `0x1004f0be0` is called with minus one.

Destroying calls the base destructor through slot `0x90` when the flags are positive, clears the stored pointer, releases the list at `0x1a0` through `0x10000b8e0`, calls the base `0x10002aae0`, and frees through slot `0x70` when the object and flags are both present.

## Model

`show_match_board` returns the stored seed and columns, the five cleared flags, the pointer and global actions and the base result. `destroy_match_board` assumes the object is present and returns the four actions from the flags.

## Open questions

The panel, base and global routines are not in.

## Docs

These methods are 379 and 121 bytes. `TMatchBoard` moves from 0% to about 3%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
