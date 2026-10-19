# Design

## Context

The draw skin is 35 KB and is rated 14% through the frame. The centred icon reads the icon size from the widget's own list or the shared list, then places the glyph.

## Goals / Non-Goals

**Goals:**

- Icon 16 in a 100 by 80 widget at 10, 20 sits at 52, 52.
- A spare of -6 sits three back, toward zero.
- A marked state with bit 0x40 sits one lower. Bits 0x60 together do not.

**Non-Goals:**

- The glyph call and the widget dispatcher.
- The colour bytes behind the ink index. The index is recorded. The blend is not in.

## Decisions

1. Half the spare is signed division toward zero, matching the shift that adds the sign bit before the arithmetic shift.
2. The marked path stores the scale bits of 1. The caption path stores 0.
3. `TGUIDrawSkin` is 16%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- The place is about 750 bytes. The dispatcher is still the large remainder of this component.

## Migration Plan

- `draw_centred_icon` places the icon. The UI plugin does not draw it yet.

## Open Questions

- The widget dispatcher is unread. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
