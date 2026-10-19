# Design

## Context

The draw skin is 35 KB and is rated 44% through the bottom bar. Mask bits 4 and 8 together, with the tab bit, then draw one strip. A clear hold bit forces mask 15, so those bits are set, and the tab bit still chooses this strip.

## Goals / Non-Goals

**Goals:**

- Unit 5, width 100 and height 80 store a strip from 80, 50 to 100, 70.
- A missing bit stores none. A clear tab bit stores none.
- A clear hold bit with the tab bit stores the wider strip, from 70, 60 to 90, 80 at height 100.

**Non-Goals:**

- The clear-tab arm.
- The helper body on the clear-hold path.

## Decisions

1. The sample is the right-cap sample, 0.65, 0.7, 0.65, 0.9.
2. On a held form the strip runs from the width minus four times the unit to the width, and from the height minus six times the unit to the height minus twice the unit. A clear hold bit starts ten units in from the width.
3. `TGUIDrawSkin` stays 44%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one strip. The clear-tab arm is not in.

## Migration Plan

- `dispatch_form` now returns the strip beside the bottom bar. The UI plugin does not draw it yet.

## Open Questions

- The clear-tab arm is unread past its first vertex. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
