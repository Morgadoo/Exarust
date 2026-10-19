# Design

## Context

The draw skin is 35 KB and is rated 19% through the dispatch opening. After the child walk the same function centres a title and, for one style, draws four corners.

## Goals / Non-Goals

**Goals:**

- A title of width 40 in a form of width 100 sits at 30. A tint byte of 200 stays 200.
- Unit 5 draws corners of span 30. Bit 2 adds a square from 100, 0 to 80, 20.
- A mark of -4 leaves that square out. Bit 0x400000 draws no corners.

**Non-Goals:**

- The edge bars that follow the four corners.
- The title glyph and the wide call's body. The wide call is counted.

## Decisions

1. Half the spare title is signed division toward zero.
2. The tint scale is 1, so the byte is unchanged.
3. `TGUIDrawSkin` is 22%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is about 1 KB more of the 11 KB dispatcher. The edge bars are not in.

## Migration Plan

- `dispatch_form` now returns the title and the corners. The UI plugin does not draw them yet.

## Open Questions

- The edge bars after the corners are unread past their first multiply. A later pass can take them the same way. No format page changes: this is draw behaviour, not a file format.
