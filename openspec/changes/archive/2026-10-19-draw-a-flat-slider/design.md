# Design

## Context

The draw skin is 35 KB and is rated 1% for its constructor. The slider draw has two modes. The clear mode builds four quads from the GUI unit. The other mode is the scroll bar.

## Goals / Non-Goals

**Goals:**

- Unit 5 at 10, 20 with width 100 and value 0 stores a thumb of 5 and a pad of 10.
- A value of one half stores a thumb of 40. A spare of 1 at one half stays on the even side.
- State bit 0x10 selects the first ink ahead of bit 0x200000.

**Non-Goals:**

- The scroll-bar quads.
- The frame, the centred icon, and the widget dispatcher.

## Decisions

1. The four samples are the constant four-float groups the flat path selects, stored as bits.
2. Rounding uses ties-to-even, matching the convert instruction's default mode.
3. `TGUIDrawSkin` is 3%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- The flat path is about 900 bytes. The scroll bar is the longer branch and is not in.

## Migration Plan

- `draw_flat_slider` is the flat path. The UI plugin does not draw it yet.

## Open Questions

- The scroll-bar quads are not transcribed. A later pass can read that branch the same way, one quad list at a time. No format page changes: this is draw behaviour, not a file format.
