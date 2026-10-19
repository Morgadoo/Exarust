# Design

## Context

The install returns at once when a ready byte is 0. An index below 1 clears the current index only when it is not already 0, shows a fallback, sets an armed byte, and stamps a clock. An index of 1 or more returns when it equals the sign-extended current index. Otherwise the index is stored. The record is the one before that index: a value, then a hook. A different non-zero value is shown only when a block word is 0. The hook runs after that show. An index past the table is stored and not read. The platform call is an indirect call. It is counted.

## Goals / Non-Goals

**Goals:**

- Index 1 with value 42 and a hook shows once and runs the hook. The same index does not show or run it again.
- A zero value still runs the hook and does not show. A block word and the same value do not show.
- Index 0 clears index 5, shows fallback 9, arms, and stamps clock 12.
- A ready byte of 0 leaves the index.

**Non-Goals:**

- Drawing through the platform call.
- A world update of the side.
- The path taken when the session-mark argument is not 0.

## Decisions

1. Records are supplied by the test. Index 1 is the first record.
2. An index past the records stores the index and does not show. The image would read past the table.
3. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.

## Risks / Trade-offs

- Counting the platform call is not the indirect call. The pointer is filled at run time.

## Migration Plan

- The counted install now runs this body. A ready byte of 0 keeps the previous tests unchanged.

## Open Questions

- None for the index compare, the record, and the show gate.
