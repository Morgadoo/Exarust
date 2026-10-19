# Design

## Context

The draw skin is 35 KB and is rated 22% through the title and the four corners. The same style then draws edge bars. A double scale of 6.6 rounds ties even to the reach.

## Goals / Non-Goals

**Goals:**

- Unit 5 on a form of 100 by 80 with no title draws a top bar from 33, 0 to 67, 10, then the left, bottom, and right bars.
- A title of width 40 splits the top at gap 17 and adds two caps.
- A style that is not 0x10, and bit 0x400000, draw no edges.

**Non-Goals:**

- The drawing that follows the right bar.
- The title glyph and the wide call's body. The wide call is counted.

## Decisions

1. The reach is unit times 6.6, rounded ties even. Unit 5 stores 33.
2. The title gap is the spare width divided by two toward zero.
3. `TGUIDrawSkin` is 25%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is 966 bytes more of the 11 KB dispatcher. The drawing after the right bar is not in.

## Migration Plan

- `dispatch_form` now returns the edge bars beside the corners. The UI plugin does not draw them yet.

## Open Questions

- The drawing after the right bar is unread. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
