# Design

## Context

The draw skin is 35 KB and is rated 46% through the quad both lower-right arms join into. When mask bits 4 and 8 are not both set, the tab bit draws another quad. A clear hold bit forces mask 15, so those bits are set and this quad is skipped.

## Goals / Non-Goals

**Goals:**

- Unit 5, width 100 and height 80 store a quad from 70, 50 to 110, 90.
- Both bits set store the bottom quad instead.
- A clear tab bit and a clear hold bit store none.

**Non-Goals:**

- The clear-tab arm.
- The helper body on the clear-hold path.

## Decisions

1. The sample is the held over sample, 0.6, 0.2, 0.6, 0.6.
2. On a held form the quad runs from the width minus six times the unit to the width plus twice the unit, and from the height minus six times the unit to the height plus twice the unit.
3. `TGUIDrawSkin` stays 46%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one quad. The clear-tab arm is not in.

## Migration Plan

- `dispatch_form` now returns the quad when those bits differ. The UI plugin does not draw it yet.

## Open Questions

- The clear-tab arm is unread past its second vertex. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
