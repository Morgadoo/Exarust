# Design

## Context

The draw skin is 35 KB and is rated 43% through the quad drawn when mask bits 2 and 8 differ. Both arms then join and draw one bar. A clear hold bit uses the wider scale.

## Goals / Non-Goals

**Goals:**

- Unit 5, width 100 and height 80 store a bar from 30, 70 to 70, 80.
- A set tab bit still stores it.
- A clear hold bit stores the wider bar, from 40, 80 to 60, 90 at height 100.

**Non-Goals:**

- The next mask test.
- The helper body on the clear-hold path.

## Decisions

1. The sample is the wide bar sample, 0.3, 0.1, 0.3, 0.2.
2. On a held form the bar runs from six times the unit to the width minus that, and from the height minus twice the unit to the height. A clear hold bit uses eight times the unit and four times the unit.
3. `TGUIDrawSkin` is 44%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one bar. The next mask test is not in.

## Migration Plan

- `dispatch_form` now returns the bar beside the skirt. The UI plugin does not draw it yet.

## Open Questions

- The next mask test is unread past its compare. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
