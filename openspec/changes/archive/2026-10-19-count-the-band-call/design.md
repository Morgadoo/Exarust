# Design

## Context

The draw skin is 35 KB and is rated 47% through the close square on the band. Both arms of that compare then pass 0 to one call and jump to the return. A clear hold bit is still on that band.

## Goals / Non-Goals

**Goals:**

- A held form and a clear hold bit each count one call.
- A close mark of -4 still counts one, and the square remains.
- A style that is not that band, and a wide bit, count none.

**Non-Goals:**

- The bytes the jump skips.
- The helper body on the clear-hold path.

## Decisions

1. The count is 1 on the band and 0 otherwise. The argument is 0.
2. The close mark does not change the count.
3. `TGUIDrawSkin` stays 47%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one count. The bytes the jump skips are not in.

## Migration Plan

- `dispatch_form` now returns the count beside the square. The UI plugin does not draw with it yet.

## Open Questions

- The jump skips a short run of bytes before the return. A later pass can read them. No format page changes: this is draw behaviour, not a file format.
