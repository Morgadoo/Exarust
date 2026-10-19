# Design

## Context

The draw skin is 35 KB and is rated 45% through the narrower clear-tab strip. Both of those arms then draw one quad. A clear hold bit forces mask 15, so those bits are set.

## Goals / Non-Goals

**Goals:**

- Unit 5, width 100 and height 80 store a quad from 70, 70 to 90, 80.
- A set tab bit still stores it, and the wider strip remains.
- A clear hold bit stores the wider quad, from 60, 80 to 80, 90 at height 100.

**Non-Goals:**

- The other arm.
- The helper body on the clear-hold path.

## Decisions

1. The sample is the held cap sample, 0.2, 0.1, 0.2, 0.2.
2. On a held form the quad runs from the width minus six times the unit to the width minus twice the unit, and from the height minus twice the unit to the height. A clear hold bit uses two, four and eight times the unit.
3. `TGUIDrawSkin` is 46%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one quad. The other arm is not in.

## Migration Plan

- `dispatch_form` now returns the quad beside the strips. The UI plugin does not draw it yet.

## Open Questions

- The other arm is unread past its tab test. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
