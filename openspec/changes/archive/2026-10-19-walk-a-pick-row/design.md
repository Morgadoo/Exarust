# Design

## Context

After a clear named check, the gate walks the actor's records from the first. A kind of 0x10010a or 0x10010d whose link is the world counts an open and returns. Any other record is skipped. The count is the list the caller supplies. The named-check body and the open callee are not in.

## Goals / Non-Goals

**Goals:**

- Kind 0x10010a linked to the world opens once, even when a later record would also match.
- Kind 0x10010d with a different link does not open.
- A set named check opens without a matching record.

**Non-Goals:**

- The pick raycast.
- The named-check body.
- The open callee.

## Decisions

1. The two kind words are record kinds, not code addresses.
2. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.

## Risks / Trade-offs

- The list is the records the test supplies. A count past that list is not represented.

## Migration Plan

- `run_gate` walks `pick_rows` when the named check is clear. The earlier named open stays.

## Open Questions

- None for the two kind words and the link compare.
