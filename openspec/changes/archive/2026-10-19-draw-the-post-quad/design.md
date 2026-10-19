# Design

## Context

The draw skin is 35 KB and is rated 35% through the lower tab. The same bit then draws one quad. Its x is the grip plus 0.15 of the unit, rounded ties even. Its low edge sits one unit below the post, and the quad is four units wide and four units tall.

## Goals / Non-Goals

**Goals:**

- A grip of 10 and a post of 40 at unit 5 stores a quad from 11, 35 to 31, 55.
- A clear bit 0x200 and a clear hold bit store none.

**Non-Goals:**

- The compare after that quad.
- The setup used when the hold bit is clear.

## Decisions

1. 0.15 of the unit is the double whose bits are 0x3fc3333333333333, rounded ties even. The quad's sample is 0.4, 0.8, 0.4, 1.
2. The left edge is that sum. The right edge adds four times the unit. The low edge subtracts the unit from the post, and the high edge adds three times the unit.
3. A close mark of -8 selects an ink slot that the sample setter replaces, so the quad colour does not change.
4. `TGUIDrawSkin` stays 35%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one quad. The compare after it is not in.

## Migration Plan

- `dispatch_form` now returns the quad beside the lower tab. The UI plugin does not draw it yet.

## Open Questions

- The compare after the quad is unread past its flag mask. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
