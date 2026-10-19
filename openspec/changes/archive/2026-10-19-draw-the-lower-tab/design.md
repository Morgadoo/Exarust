# Design

## Context

The draw skin is 35 KB and is rated 34% through the held tab. The next quad uses the same x and hangs below a seat. Mask bits 0xc together subtract three times the unit from the seat. Otherwise they subtract twice the unit.

## Goals / Non-Goals

**Goals:**

- A seat of 80 at unit 5 stores a tab from 16, 60 to 26, 70.
- Mask bits 4 and 8 together store it from 16, 55 to 26, 65.
- A clear bit 0x200 and a clear hold bit store none.

**Non-Goals:**

- The anchor recomputed after this quad.

## Decisions

1. The x is the same grip plus 0.15 of the unit used by the upper tab.
2. The quad is two units wide and hangs two units below its anchor.
3. `TGUIDrawSkin` is 35%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is the lower tab only. The next anchor is not in.

## Migration Plan

- `dispatch_form` now returns the lower tab beside the upper one. The UI plugin does not draw it yet.

## Open Questions

- The anchor after this tab is unread past its first reload of the grip. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
