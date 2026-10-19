# Design

## Context

The draw skin is 35 KB and is rated 3% for its constructor and the flat slider. The scroll mode builds a trough, then twelve quads from the unit and a stored lip.

## Goals / Non-Goals

**Goals:**

- Unit 5 at 10, 20, size 100 by 80 and lip 4 stores trough 16, 26, 88, 68.
- Value 0 stores thumbs 10 and 25. Value one half stores thumb 52.
- A spare of 1 at one half stays on the even side.

**Non-Goals:**

- The frame, the centred icon, and the widget dispatcher.
- The colour call's byte mix. The bar bit is recorded. Its blend is not in.

## Decisions

1. The twelve samples are the two constant four-float groups that mode selects, stored as bits.
2. Rounding uses ties-to-even, the same convert the flat thumb uses.
3. `TGUIDrawSkin` is 10%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- The scroll branch is about 2.4 KB. The frame and the dispatcher are still the larger remainder.

## Migration Plan

- `draw_flat_slider` draws the scroll bar when the high mode bits are 0x10000000. The UI plugin does not draw it yet.

## Open Questions

- The frame's corner squares and the widget dispatcher are unread. A later pass can take one of those the same way. No format page changes: this is draw behaviour, not a file format.
