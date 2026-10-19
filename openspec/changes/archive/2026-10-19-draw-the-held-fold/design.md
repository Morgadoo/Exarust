# Design

## Context

The draw skin is 35 KB and stays 46% through the tab-set quad drawn when mask bits 4 and 8 differ. A clear tab bit draws the same corners with another sample. A clear hold bit forces mask 15, so those bits are set and this quad is skipped.

## Goals / Non-Goals

**Goals:**

- Unit 5, width 100 and height 80 store a quad from 70, 50 to 110, 90.
- A set tab bit stores the other sample instead.
- Both bits set, and a clear hold bit, store none.

**Non-Goals:**

- The join after that quad.
- The helper body on the clear-hold path.

## Decisions

1. The sample is the held flip sample, 0.15, 0.2, 0.15, 0.6.
2. On a held form the quad runs from the width minus six times the unit to the width plus twice the unit, and from the height minus six times the unit to the height plus twice the unit.
3. `TGUIDrawSkin` stays 46%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one quad. The join after it is not in.

## Migration Plan

- `dispatch_form` now returns the quad beside the tab-set one. The UI plugin does not draw it yet.

## Open Questions

- The join stores new scales and then calls onward. A later pass can take that the same way. No format page changes: this is draw behaviour, not a file format.
