# Design

## Context

The draw skin is 35 KB and is rated 31% through the ink copy. The join flushes the batch, then draws one square when the close mark is -4. The sample is 0, 0.8, 0, 1. The span is four times the unit.

## Goals / Non-Goals

**Goals:**

- Unit 5 and width 100 with a mark of -4 store a square from 100, 0 to 80, 20.
- A mark of 0 or -3 stores none.

**Non-Goals:**

- The hold-bit edge test on the path that is not wide.
- The batch flush itself. It ends the quads and adds no square.

## Decisions

1. The square uses the same sample as the earlier close square.
2. The mark test is equality with -4. Neighbouring values do not match.
3. `TGUIDrawSkin` is 32%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is the square at the join. The hold-bit edge test is not in.

## Migration Plan

- `dispatch_form` now returns the late square. The UI plugin does not draw it yet.

## Open Questions

- The hold-bit edge test is unread past its first compares. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
