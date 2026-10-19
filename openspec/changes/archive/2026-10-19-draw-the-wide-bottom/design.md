# Design

## Context

The draw skin is 35 KB and is rated 28% through the wide left side. The same gate then draws a bottom: a bar between twice the unit and the width minus that, sitting on the height, and a cap at each end.

## Goals / Non-Goals

**Goals:**

- Unit 5, width 100 and height 80 store a bar from 10, 75 to 90, 85, a cap from 0, 75 to 10, 85, and a cap from 90, 75 to 100, 85.
- Style 0x10 without bits 0x80 draws none. A form that is not wide draws none.

**Non-Goals:**

- The right side that follows this bottom.

## Decisions

1. The bottom uses the same band and the same two samples as the top, shifted by the height.
2. The gate stays bits 0xc0 of the style word equal to 0x80, and the wide bit.
3. `TGUIDrawSkin` is 29%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is the bottom only. The right side is not in.

## Migration Plan

- `dispatch_form` now returns the bottom beside the left side. The UI plugin does not draw it yet.

## Open Questions

- The right side is unread past its first vertex. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
