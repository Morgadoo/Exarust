# Design

## Context

The draw skin is 35 KB and stays 35% through the post quad. A held form then draws a left strip. Its x runs from 0 to twice the unit. Its y runs from six times the unit up to the height minus that same reach.

## Goals / Non-Goals

**Goals:**

- Unit 5 and height 80 store a strip from 0, 30 to 10, 50.
- A clear bit 0x200 still stores it.
- A clear hold bit and a wide bit store none.

**Non-Goals:**

- The mask compare after that strip.
- The setup used when the hold bit is clear.

## Decisions

1. The strip uses the same sample as the wide bar: 0.3, 0.1, 0.3, 0.2.
2. The reach is six times the unit. The band is twice the unit. The top subtracts the reach from the height.
3. A flag word of 0x44 selects an ink slot that the sample setter replaces, so the strip colour does not change.
4. `TGUIDrawSkin` is 36%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one strip. The compare after it is not in.

## Migration Plan

- `dispatch_form` now returns the strip beside the post quad. The UI plugin does not draw it yet.

## Open Questions

- The compare after the strip is unread past its flag mask. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
