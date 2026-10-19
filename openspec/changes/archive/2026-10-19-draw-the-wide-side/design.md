# Design

## Context

The draw skin is 35 KB and is rated 26% through the wide top. The same gate then draws a left side: a bar between twice the unit and the height minus that, and a cap at each end.

## Goals / Non-Goals

**Goals:**

- Unit 5 and height 80 store a bar from -5, 10 to 5, 70, a cap from y 0 to 10, and a cap from y 70 to 80.
- Style 0x10 without bits 0x80 draws none. A form that is not wide draws none.

**Non-Goals:**

- The bottom that follows this side.
- The right side. It is later in the same frame.

## Decisions

1. The side uses the same band, twice the unit, and the same two samples as the top.
2. The gate stays bits 0xc0 of the style word equal to 0x80, and the wide bit.
3. `TGUIDrawSkin` is 28%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is the left side only. The bottom and the right side are not in.

## Migration Plan

- `dispatch_form` now returns the left side beside the wide top. The UI plugin does not draw it yet.

## Open Questions

- The bottom after this side is unread past its first vertex. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
