# Design

## Context

The draw skin is 35 KB and is rated 10% for its constructor, the flat slider, and the scroll bar. The frame draw builds four corners and two edge bars, then two side bars when the frame is tall.

## Goals / Non-Goals

**Goals:**

- Unit 5 at 10, 20 and size 100 by 20 stores six quads and ink 2.
- Height 80 stores eight quads. Height 20 does not add the side bars.
- State bit 0x20 selects ink 3. State bit 0x10 selects ink 0.

**Non-Goals:**

- The centred icon and the widget dispatcher.
- The colour call's byte body. The ink index is recorded. The blend is not in.

## Decisions

1. Corner and edge samples are the two constant four-float groups, stored as bits.
2. The side test compares four times the unit with the height as signed values.
3. `TGUIDrawSkin` is 14%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- The frame is about 1.5 KB. The dispatcher is still the large remainder of this component.

## Migration Plan

- `draw_frame` is the frame draw. The UI plugin does not draw it yet.

## Open Questions

- The centred icon and the widget dispatcher are unread. A later pass can take one of those the same way. No format page changes: this is draw behaviour, not a file format.
