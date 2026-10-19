# Design

## Context

The draw skin is 35 KB and is rated 37% through the sign flip. Mask bits 1 and 4 together, with bit 0x200, then draw a cap at the right of the form. Its x runs from the width minus six times the unit to the width. Its y runs from twice the unit to six times the unit.

## Goals / Non-Goals

**Goals:**

- Unit 5 and width 100 store a cap from 70, 10 to 100, 30.
- A clear bit 0x200 stores none.
- An edge that misses either bit, and a clear hold bit, store none.

**Non-Goals:**

- The quad drawn when bit 0x200 is clear.
- The helper body on the clear-hold path.

## Decisions

1. The sample is 0.65, 0.7, 0.65, 0.9.
2. The inner edge subtracts six times the unit from the width.
3. `TGUIDrawSkin` is 38%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one cap. The quad after a clear tab bit is not in.

## Migration Plan

- `dispatch_form` now returns the cap beside the sign-flipped quad. The UI plugin does not draw it yet.

## Open Questions

- The clear-tab quad is unread past its first vertices. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
