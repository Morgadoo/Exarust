# Design

## Context

The draw skin is 35 KB and is rated 36% through the cap. Mask bits 1 and 2 together then draw a ledge. Its x runs from twice the unit to six times the unit, and its y runs from 0 to twice the unit.

## Goals / Non-Goals

**Goals:**

- Unit 5 with both edge bits stores a ledge from 10, 0 to 30, 10.
- Bit 1, or owner bit 1, stores none unless the owner also has bit 0x800000.
- An edge of 5, 5 and a clear hold bit store none.

**Non-Goals:**

- The sign flip that runs when those mask bits are not both set.
- The setup used when the hold bit is clear.

## Decisions

1. The ledge uses the same sample as the cap: 0.2, 0.1, 0.2, 0.2.
2. The skip is the owner's high bit being clear while bit 1 or the owner's low bit is set.
3. `TGUIDrawSkin` is 37%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one ledge. The sign flip and the clear-hold setup are not in.

## Migration Plan

- `dispatch_form` now returns the ledge beside the cap. The UI plugin does not draw it yet.

## Open Questions

- The sign flip is unread past its first vertices. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
