# Design

## Context

The draw skin is 35 KB and is rated 37% through the ledge. The same band, with no wide bit, reaches a different setup when the hold bit is clear. That setup forces the mask to 15 and uses a wider scale: the strip starts at twice the unit and is four units wide, and its low edge is eight times the unit.

## Goals / Non-Goals

**Goals:**

- Unit 5 and height 100 store a strip from 10, 40 to 20, 60, a cap from 10, 20 to 20, 40, and a ledge from 20, 10 to 40, 20.
- A clear bit 1 counts one call. A set bit counts none and skips the ledge.
- Bit 0x200 raises the tab so its top is 45. The held mask stays 0.

**Non-Goals:**

- The helper body.
- The sign flip used when the held mask bits are not both set.

## Decisions

1. The forced mask is stored apart from the held mask, which stays 0 when the hold bit is clear.
2. The strip uses the wide-bar sample. The cap and the ledge use the cap sample.
3. `TGUIDrawSkin` stays 37%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- The call is counted. Its body is not in.

## Migration Plan

- `dispatch_form` now returns the wider frame beside the held frame. The UI plugin does not draw it yet.

## Open Questions

- The helper body is unread. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
