# Design

## Context

The draw skin is 35 KB and is rated 33% through the held bar. Bit 0x200 then draws one tab. Its x adds the unit to a grip plus 0.15 of the unit, rounded ties even. Its y sits on the grip plus twice the unit, or three times when mask bits 1 and 4 are both set.

## Goals / Non-Goals

**Goals:**

- A grip of 10, 20 at unit 5 stores a tab from 16, 30 to 26, 40.
- Mask bits 1 and 4 together store it from 16, 35 to 26, 45.
- A clear bit 0x200 and a clear hold bit store none.

**Non-Goals:**

- The next anchor, which uses mask bits 0xc.
- The setup used when the hold bit is clear.

## Decisions

1. 0.15 of the unit is the double whose bits are 0x3fc3333333333333, rounded ties even.
2. The tab is two units wide and two units tall.
3. `TGUIDrawSkin` is 34%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one tab. The next anchor is not in.

## Migration Plan

- `dispatch_form` now returns the held tab beside the bar. The UI plugin does not draw it yet.

## Open Questions

- The next anchor is unread past its first compare. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
