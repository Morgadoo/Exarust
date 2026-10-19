# Design

## Context

The draw skin is 35 KB and is rated 38% through the wider right cap. The same mask, with bit 0x200 clear, draws a narrower cap. Its x runs from the width minus twice the unit to the width. Its y runs from twice the unit to six times the unit.

## Goals / Non-Goals

**Goals:**

- Unit 5 and width 100 store a cap from 90, 10 to 100, 30.
- A set bit 0x200 stores the wider cap and not this one.
- An edge that misses either bit, and a clear hold bit, store none.

**Non-Goals:**

- The flag test after that cap.
- The helper body on the clear-hold path.

## Decisions

1. The cap uses the same sample as the left cap: 0.2, 0.1, 0.2, 0.2.
2. The inner edge subtracts twice the unit from the width.
3. `TGUIDrawSkin` stays 38%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one cap. The flag test after it is not in.

## Migration Plan

- `dispatch_form` now returns the narrow cap beside the wider one. The UI plugin does not draw it yet.

## Open Questions

- The flag test is unread past its first compare. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
