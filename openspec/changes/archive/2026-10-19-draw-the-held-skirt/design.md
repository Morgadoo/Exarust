# Design

## Context

The draw skin is 35 KB and is rated 43% through the ledge. When mask bits 2 and 8 are not both set, the other arm draws one quad. A clear hold bit forces mask 15 and takes the ledge instead.

## Goals / Non-Goals

**Goals:**

- Unit 5 and height 80 store a quad from -10, 50 to 30, 90.
- Both bits set store the ledge instead.
- A clear hold bit stores none.

**Non-Goals:**

- The next quad.
- The helper body on the clear-hold path.

## Decisions

1. The sample is the sign-flip sample, 0.15, 0.2, 0.15, 0.6.
2. The left x negates twice the unit. The right x is six times the unit. The y runs from the height minus six times the unit to the height plus twice the unit.
3. `TGUIDrawSkin` stays 43%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one quad. The next quad is not in.

## Migration Plan

- `dispatch_form` now returns the quad beside the ledge. The UI plugin does not draw it yet.

## Open Questions

- The next quad is unread past its first vertex. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
