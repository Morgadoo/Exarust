# Design

## Context

A contact knocks a dormant sleeper out at the current clock, with tone 0 and the waking flag set. Each later tick while that flag is set raises the tone.

## Goals / Non-Goals

**Goals:**

- Hold tone at 0 through 1500 ms, with clip base 0.4 and condition down.
- Rise by 0.001 a tick after that, plus 0.001·(1 − phase)² while getting up.
- Climb the clip base by 0.01 a tick after 8 s, and finish at tone 1.

**Non-Goals:**

- The awaken script and the weapon refresh.
- The get-up the posture block starts.

## Decisions

1. The waits are strict. 1500 ms and 8000 ms are still the earlier branch. The clocks widen before the add.
2. The stored target is written only in the first 1500 ms, where it is 0, and when the clip base reaches 1, where it becomes the stance factor. The 0.001 rise does not write it.
3. Past 1500 ms the dormant flag clears, unless the held flag is set, in which case the dormant, waking and held flags are all set. At full strength the low flag nibble clears unless held.
4. The tone add is done in double precision and then stored, including the getting-up square.

## Risks / Trade-offs

- The rise is not yet called from the body tick. The body's derived tone target is tone times the stance factor, so during the rise it is not the stored 0.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The waking tick was read before this change.
