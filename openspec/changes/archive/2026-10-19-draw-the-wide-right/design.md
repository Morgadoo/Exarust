# Design

## Context

The draw skin is 35 KB and is rated 29% through the wide bottom. The same gate then draws a right side: a bar between twice the unit and the height minus that, placed one unit either side of the width, and a cap at each end.

## Goals / Non-Goals

**Goals:**

- Unit 5, width 100 and height 80 store a bar from 95, 10 to 105, 70, a cap from y 0 to 10, and a cap from y 70 to 80.
- Style 0x10 without bits 0x80 draws none. A form that is not wide draws none.

**Non-Goals:**

- The mark that follows this side. Bit 0x40 of the next word either leaves or sets an ink. That body is not in.

## Decisions

1. The right side mirrors the left, with the width added to each x.
2. The gate stays bits 0xc0 of the style word equal to 0x80, and the wide bit.
3. `TGUIDrawSkin` is 31%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This closes the four edges of the wide frame. The mark after them is not in.

## Migration Plan

- `dispatch_form` now returns the right side beside the bottom. The UI plugin does not draw it yet.

## Open Questions

- The mark after this side is unread past its first test. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
