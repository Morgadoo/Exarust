# Design

## Context

The draw skin is 35 KB and is rated 42% through the clear-tab right strip. The join then draws one strip when mask bits 2 and 8 are both set. A clear hold bit forces mask 15, so those bits are set.

## Goals / Non-Goals

**Goals:**

- Unit 5 and height 80 store a strip from 0, 50 to 10, 70.
- A missing bit stores none. A set tab bit still stores it.
- A clear hold bit stores the wider strip, from 10, 60 to 20, 80 at height 100.

**Non-Goals:**

- The next quad.
- The helper body on the clear-hold path.

## Decisions

1. The sample is the held cap sample, 0.2, 0.1, 0.2, 0.2.
2. On a held form the strip runs from x 0 to twice the unit, and from the height minus six times the unit to the height minus twice the unit. A clear hold bit starts at twice the unit and uses four and eight times the unit.
3. `TGUIDrawSkin` stays 42%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one strip. The next quad is not in.

## Migration Plan

- `dispatch_form` now returns the strip beside the clear-tab strip. The UI plugin does not draw it yet.

## Open Questions

- The next quad is unread past its first vertices. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
