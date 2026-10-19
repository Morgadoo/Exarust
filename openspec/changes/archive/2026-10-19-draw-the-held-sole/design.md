# Design

## Context

The draw skin is 35 KB and is rated 42% through the lower strip. The same mask bits then draw one ledge and jump past the other arm. A clear hold bit forces mask 15, so those bits are set.

## Goals / Non-Goals

**Goals:**

- Unit 5 and height 80 store a ledge from 10, 70 to 30, 80.
- A missing bit stores none. A set tab bit still stores it.
- A clear hold bit stores the wider ledge, from 20, 80 to 40, 90 at height 100.

**Non-Goals:**

- The other arm of that compare.
- The helper body on the clear-hold path.

## Decisions

1. The sample is the held cap sample, 0.2, 0.1, 0.2, 0.2.
2. On a held form the ledge runs from twice the unit to six times the unit, and from the height minus twice the unit to the height. A clear hold bit uses four and eight times the unit.
3. `TGUIDrawSkin` is 43%. The UI section stays 0%. The published headline stays 14.4% and 5.7%. The fresh sum stays about 15.5%.

## Risks / Trade-offs

- This is one ledge. The other arm is not in.

## Migration Plan

- `dispatch_form` now returns the ledge beside the lower strip. The UI plugin does not draw it yet.

## Open Questions

- The other arm is unread past its first vertex. A later pass can take it the same way. No format page changes: this is draw behaviour, not a file format.
