# Design

## Behaviour

The requested frame is the object's `0x190`. Below 1 the method ends. The child at `0xa0` holds the current frame at `0x158 + 0x40`; when it already equals the requested frame less one, the method ends without a refresh.

Otherwise the child's frame becomes the requested less one. The refresh `0x1000ef7f0` runs with the child's `0x168`, the part search `0x10012cb10` runs on `0x168 + 0x18`, and its result is stored at the `0x180` object's `0x180`.

## Model

`select_creation_frame(requested, current, part_found)` returns whether the frame changed, the resulting frame, whether the part was found and whether the refresh ran. The part result is supplied because the search is shipped separately.

## Open questions

The refresh body is not in.

## Docs

This method is 128 bytes. `SG_Character_Create` moves from 12% to about 13%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.
