# Design

## Context

After a taken item returns, the gate looks up the item id. Id 0 misses. Bit 0x80000000 selects the high table. Else bit 0x40000000 selects the wide table. Otherwise the plain table is used. A high or wide id is masked with 0x3fffffff. The rows are sorted. The search takes the low plus half the distance, with the sign bit added before the shift. The key is the first word of a 16-byte row. A size of 0 is not read. The span is little-endian records of a tag and a step. Tag 0x20000 is a hit. A step of 0 stops, so a stuck walk cannot spin. A span past the supplied bytes is not read.

## Goals / Non-Goals

**Goals:**

- Id 7 in the plain table with the mark counts one kind call.
- Id 0x80000005 searches the high table for key 5.
- Id 0x40000009 searches the wide table for key 9, not the plain row of the unmasked id.
- Both high bits still search the high table.

**Non-Goals:**

- The two calls after a hit.
- The pick raycast.
- Loading retail kind bytes.

## Decisions

1. The three tables are supplied by the caller. Tests use synthetic bytes.
2. A step of 0 stops. The image would spin.
3. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.

## Risks / Trade-offs

- A zero step is a stop, not a spin.

## Migration Plan

- `run_gate` calls the lookup instead of treating any non-zero id as a hit. The old kind test supplies a one-row table.

## Open Questions

- None for the table choice, the search, and the mark.
