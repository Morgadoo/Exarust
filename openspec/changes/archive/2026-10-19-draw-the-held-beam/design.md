# Design

## Context

The draw skin is 35 KB and is rated 40% through the right strip. The plain word is the same word whose bit 0x100 skips the shift. Its bit 0x200 skips the next bar. The bar is reached only when the tab bit is set. A clear hold bit stores mask 15, so its bit 4 skips the bar.

## Goals / Non-Goals

**Goals:**

- Unit 5, width 100 and height 80 store a bar from 96, 20 to 106, 60.
- Bit 0x200 of the plain word stores none, and the right strip remains.
- Mask bit 4, a clear tab bit, and a clear hold bit store none.

**Non-Goals:**

- The drawing after that bar.
- The helper body on the clear-hold path.

## Decisions

1. The sample is the wide bar sample, 0.3, 0.1, 0.3, 0.2.
2. The x origin is the width plus the unit times 0.2, rounded ties even. The bar is two units wide and runs from four times the unit to the height minus that.
3. `TGUIDrawSkin` is 41%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one bar. The drawing after it is not in.

## Migration Plan

- `dispatch_form` now returns the bar beside the right strip. The UI plugin does not draw it yet.

## Open Questions

- The drawing after that bar is unread past its first vertices. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
