# Design

## Context

The penalty byte is reserved for footprints and props. The obstacle test already refuses a penalty above 60. Nothing writes a footprint yet.

## Goals / Non-Goals

**Goals:**

- Stamp and remove the 63 / 32 / 16 pattern on the matching layer.
- Show that the obstacle test sees the centre and forgets it when the footprint is removed.

**Non-Goals:**

- The mover refresh and dirty rectangles.
- Logging the once-only overflow message.

## Decisions

1. The lower layer matches when its underside is within three below the height and its floor is within twenty-five above. Otherwise the upper layer is tried. No match writes nothing.
2. The centre is 255, each neighbour 128, and each outer cell 64, added to the penalty as those values divided by four. Removal subtracts the same bytes.
3. A centre in the outer five cells, on either axis, is not stamped.

## Risks / Trade-offs

- A second stamp on the same cells adds again. The byte wraps, as a byte add does.

## Migration Plan

No data migration. No format page: no field changes.

## Open Questions

- None for this slice. The stamp, the rings, and the quartered penalty were read before this change.
