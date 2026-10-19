# Design

## Context

The draw skin is 35 KB and is rated 16% through the centred icon. The dispatcher is about 11 KB. Its opening stores three spans, then optionally shades, fills a quad, shifts the scroll, and visits shown children.

## Goals / Non-Goals

**Goals:**

- Unit 5 stores spans 10, 15 and 20.
- A spare of 30 at value 1 shifts by 30. A spare of 1 at one half stays even.
- Children marked 0, 1, 0, 1 visit 1 and 3.

**Non-Goals:**

- The drawing that follows the child walk.
- The shade call's body and the clip call's body. Those calls are counted.

## Decisions

1. The shift uses ties-to-even, the same convert the slider thumb uses.
2. A child list shorter than the count is not read past its end.
3. `TGUIDrawSkin` is 19%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- The opening is about 1 KB of an 11 KB function. The remainder is not in.

## Migration Plan

- `dispatch_form` is the opening. The UI plugin does not draw with it yet.

## Open Questions

- The drawing after the child walk is unread. A later pass can take the next branch the same way. No format page changes: this is draw behaviour, not a file format.
