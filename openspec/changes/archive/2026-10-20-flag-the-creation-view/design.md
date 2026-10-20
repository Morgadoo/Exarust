# Design

## Behaviour

The refresh calls `0x1000f13c0` with the child's `0x168` and `0x158`, then `0x100133020` with the object's `0x160` and the child's `0x168`, then `0x100071a20`, then `0x1001e5ea0`.

The flag test is the option word at `0x2` equal to `0x11`. When it matches, bit `0x10000` is ORed into the child's `0x54`; otherwise the bit is cleared with the `0xfffeffff` mask. The refresh `0x1000ef7f0` then runs with the child's `0x168`, the part search `0x10012cb10` runs on `0x168 + 0x18`, and its result is stored at the `0x180` object's `0x180`.

## Model

`refresh_creation_view()` returns the four call flags. `set_creation_flag(word, flag, part_found)` returns the updated word, the refresh flag and the stored part result.

## Open questions

The four refresh calls and the part search body are not in.

## Docs

These methods are 65 and 108 bytes. `SG_Character_Create` moves from 20% to about 21%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
