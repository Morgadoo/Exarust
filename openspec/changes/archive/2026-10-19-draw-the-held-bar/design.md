# Design

## Context

The draw skin is 35 KB and is rated 32% through the hold-bit mask. On that same path the next quad is one bar. Its x is four times the unit plus twice the unit. Its thickness is twice the unit.

## Goals / Non-Goals

**Goals:**

- Unit 5 and width 100 store a bar from 30, 0 to 70, 10.
- A y below the unit with owner bit 1 and without bit 0x800000 stores none. Owner bit 0x800000 stores the bar.

**Non-Goals:**

- The drawing after that bar.
- The setup used when the hold bit is clear.

## Decisions

1. The reach is the sum of four times the unit and twice the unit.
2. The skip needs mask bit 1, owner bit 1, and a clear owner bit 0x800000.
3. `TGUIDrawSkin` is 33%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one bar. The drawing after it is not in.

## Migration Plan

- `dispatch_form` now returns the held bar beside the mask. The UI plugin does not draw it yet.

## Open Questions

- The drawing after the bar is unread past its first flag test. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
