# Design

## Context

The draw skin is 35 KB and is rated 41% through the bar after the right strip. The same gate then draws two caps and joins the skip. The clear tab path is a different entry.

## Goals / Non-Goals

**Goals:**

- Unit 5, width 100 and height 80 store an upper cap from 96, 0 to 106, 20 and a lower cap from 96, 60 to 106, 80.
- The plain bit, mask bit 4, a clear tab bit, and a clear hold bit store none.

**Non-Goals:**

- The drawing on the clear tab path.
- The helper body on the clear-hold path.

## Decisions

1. Both caps use the wide cap sample, 0.7, 0.1, 0.7, 0.2.
2. Each cap is two units wide. The upper cap runs from the hold slot to four times the unit. The lower cap runs from the height minus four times the unit to the height.
3. `TGUIDrawSkin` is 42%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- These are two caps. The clear-tab drawing is not in.

## Migration Plan

- `dispatch_form` now returns the caps beside the bar. The UI plugin does not draw them yet.

## Open Questions

- The clear-tab drawing is unread past its first vertex. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
