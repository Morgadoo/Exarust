# Design

## Context

The draw skin is 35 KB and is rated 44% through the lower-right strip. A clear tab bit on that same mask draws a narrower strip. A clear hold bit forces mask 15, so those bits are set.

## Goals / Non-Goals

**Goals:**

- Unit 5, width 100 and height 80 store a strip from 90, 50 to 100, 70.
- A set tab bit stores the wider strip instead.
- A clear hold bit with a clear tab bit stores the wider clear strip, from 80, 60 to 90, 80 at height 100.

**Non-Goals:**

- The next quad.
- The helper body on the clear-hold path.

## Decisions

1. The sample is the held cap sample, 0.2, 0.1, 0.2, 0.2.
2. On a held form the strip runs from the width minus twice the unit to the width, and from the height minus six times the unit to the height minus twice the unit. A clear hold bit uses four and eight times the unit.
3. `TGUIDrawSkin` is 45%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one strip. The next quad is not in.

## Migration Plan

- `dispatch_form` now returns the strip beside the wider one. The UI plugin does not draw it yet.

## Open Questions

- The next quad is unread past its sample load. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
