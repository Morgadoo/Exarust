# Design

## Context

The draw skin is 35 KB and is rated 32% through the late close square. On the path that is not wide, the hold bit compares the form with the unit and with the room.

## Goals / Non-Goals

**Goals:**

- Unit 5, y 4 and x 4 set mask bits 1 and 2. A right sum of 109 past a room of 100 sets bit 4.
- An equal sum sets neither extra bit. A clear hold bit, a wide bit, and style 0x10 set none.

**Non-Goals:**

- The quads that follow the mask.

## Decisions

1. Below the unit is a signed less-than. Past the room is a signed greater-than of the 64-bit sum.
2. The room is the far end minus the near end.
3. `TGUIDrawSkin` stays 32%. The added bytes do not cross the next point. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is the mask only. The quads that use it are not in.

## Migration Plan

- `dispatch_form` now returns the edge mask. The UI plugin does not draw from it yet.

## Open Questions

- The quads after the mask are unread past their first colour. A later pass can take them the same way. No format page changes: this is draw behaviour, not a file format.
