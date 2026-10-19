# Design

## Context

The draw skin is 35 KB and is rated 36% through the left strip. Mask bits 1 and 2 together then draw a cap. Its x runs from 0 to twice the unit. Its y runs from twice the unit to six times the unit.

## Goals / Non-Goals

**Goals:**

- Unit 5 with both edge bits stores a cap from 0, 10 to 10, 30.
- An edge of 5, 5 stores none, and the left strip remains.
- A clear hold bit stores none.

**Non-Goals:**

- The flag test after that cap.
- The setup used when the hold bit is clear.

## Decisions

1. The cap sample is 0.2, 0.1, 0.2, 0.2.
2. The near edge is twice the unit. The far edge is six times the unit.
3. `TGUIDrawSkin` stays 36%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one cap. The flag test after it is not in.

## Migration Plan

- `dispatch_form` now returns the cap beside the left strip. The UI plugin does not draw it yet.

## Open Questions

- The flag test after the cap is unread past its first compare. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
