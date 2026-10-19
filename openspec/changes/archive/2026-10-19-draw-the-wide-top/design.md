# Design

## Context

The draw skin is 35 KB and is rated 25% through the edge bars. A wide form skips those bars and, when bits 0xc0 of the style word equal 0x80, draws a top of two caps and one bar.

## Goals / Non-Goals

**Goals:**

- Unit 5 and width 100 store a left cap from 0, -5 to 10, 5, a bar from 10, -5 to 90, 5, and a right cap from 90, 5 to 100, -5.
- Style 0x10 without bits 0x80 draws none of these. A form that is not wide draws none.

**Non-Goals:**

- The side bars that follow this top.
- The float stores that follow the right edge bar on the path that is not wide.

## Decisions

1. The band is twice the unit. The upper edge is the negation of the unit.
2. The gate is the style word, independent of the style bits 0x10.
3. `TGUIDrawSkin` is 26%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is the top of the wide frame only. The sides and the bottom are not in.

## Migration Plan

- `dispatch_form` now returns the wide top beside the corners. The UI plugin does not draw it yet.

## Open Questions

- The side bars after this top are unread. A later pass can take them the same way. No format page changes: this is draw behaviour, not a file format.
