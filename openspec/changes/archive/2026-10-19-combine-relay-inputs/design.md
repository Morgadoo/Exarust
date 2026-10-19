# Design

## Context

Dispatch already names relay kinds 0, 1, 2, 3 and 5. This change runs 0, 2, 3 and 5.

## Goals / Non-Goals

**Goals:**

- Set one bit and compare the state with a code.
- Forward one value to both targets.
- Count eight inputs and reset.

**Non-Goals:**

- The broadcast that walks every sector.

## Decisions

1. The bit index is masked to five bits, and only the low 16 bits of the result are kept. A missing source does not change the combiner.
2. Invert swaps the match value. Silence drops only the mismatch. The second combiner uses the state's low bit, not the silence flag, and forwards the original value.
3. A sequence value of 0 does nothing. Mark 0 finishes. The count lives in the high nibble. Overlap with the previous high byte sets the wrong-order flag. Finish keeps the low nibble, clears the count and the flag, and clears a listener only when its class bits match and it lacks the skip flag.

## Risks / Trade-offs

- Nothing in the world calls this yet. The functions are the relay rules.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The four relay steps were read before this change.
