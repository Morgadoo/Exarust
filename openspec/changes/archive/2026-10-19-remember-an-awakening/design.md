# Design

## Context

The wake tick already drops the dormant flag after 1500 ms and counts who can see. This slice writes the memory word that count was missing. The seconds clock is not the wake millisecond clock.

## Goals / Non-Goals

**Goals:**

- Update the memory word of each person who can see, on the tick that drops the dormant flag.
- Honour the cooldown, the delay, the count, the witness mask, and the activated window.

**Non-Goals:**

- The role-by-role dialogue walk and its invalid-role report.
- The grip pass and the arm-mass scale inside the weapon refresh.
- Steering along an accepted path.

## Decisions

1. World time is a seconds field on the wake, separate from the millisecond clock. Comparisons that add or subtract times widen to 64 bits.
2. The low 30 bits are the stamp, the count, or the witness mask. Bit 30 means counting. Bit 31 means open. A witness mask counts only those low 30 bits.
3. A never-again nibble returns before the cooldown table. Index 0 of that table is the activation window that does not expire inside a 32-bit second.
4. An update can report changed when the word is the same. That is the open stamp that is still in the future. Held-off on an open word reports unchanged.

## Risks / Trade-offs

- The notice still does not walk roles. A witness carries its own node flags and witness bits.

## Migration Plan

No data migration. No format page: no field changes.

## Open Questions

- None for this slice. The updater, the bit count, and both tables were read before this change.
