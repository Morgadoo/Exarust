# Design

## Context

The draw skin is 35 KB and is rated 38% through the narrow cap. The same clear tab bit then draws a shelf. Its x runs from the width minus six times the unit to the width minus twice the unit. Its y runs from 0 to twice the unit.

## Goals / Non-Goals

**Goals:**

- Unit 5 and width 100 store a shelf from 70, 0 to 90, 10.
- Bit 1, or owner bit 1, stores none unless the owner also has bit 0x800000.
- A set bit 0x200, a missing edge bit, and a clear hold bit store none.

**Non-Goals:**

- The quad that follows that shelf.
- The helper body on the clear-hold path.

## Decisions

1. The shelf uses the same sample as the narrow cap: 0.2, 0.1, 0.2, 0.2.
2. The skip matches the left ledge: the owner's high bit is clear while bit 1 or the owner's low bit is set.
3. `TGUIDrawSkin` is 39%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one shelf. The quad after it is not in.

## Migration Plan

- `dispatch_form` now returns the shelf beside the narrow cap. The UI plugin does not draw it yet.

## Open Questions

- The next quad is unread past its first compare. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
