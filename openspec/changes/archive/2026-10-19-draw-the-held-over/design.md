# Design

## Context

The draw skin is 35 KB and is rated 39% through the shelf. When mask bits 1 and 4 are not both set, bit 0x200 draws a quad past the right edge. Its x runs from the width minus six times the unit to the width plus twice the unit. Its y runs from the negation of twice the unit to six times the unit.

## Goals / Non-Goals

**Goals:**

- Unit 5 and width 100 store a quad from 70, -10 to 110, 30.
- A clear bit 0x200 stores none.
- An edge of 4, 4 with a right room of 100 stores the wider cap instead.

**Non-Goals:**

- The quad drawn when that tab bit is clear.
- The helper body on the clear-hold path.

## Decisions

1. The sample is 0.6, 0.2, 0.6, 0.6.
2. The outer x adds twice the unit to the width. The low y negates twice the unit.
3. `TGUIDrawSkin` stays 39%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one quad. The clear-tab quad on that path is not in.

## Migration Plan

- `dispatch_form` now returns the quad beside the shelf. The UI plugin does not draw it yet.

## Open Questions

- The clear-tab quad is unread past its first vertex. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
