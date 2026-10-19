# Design

## Context

A walker already toggles a door that is at most 0.7 open. The path step's door phase is what decides to leave that wait.

## Goals / Non-Goals

**Goals:**

- Leave any other phase alone.
- Move past the door when the door check lets the walker through, and let the search run.
- Give up strictly after ten seconds: seek again, clear the avoided node, and skip the cursor's low five bits. That rule wins the phase when both happen.

**Non-Goals:**

- The heading sway that runs before the door check.
- The other path phases.

## Decisions

1. Only the door phase steps. The door check is the function already shipped.
2. Ten seconds is `start + 10000 < now`, with both clocks widened before the add, so a small now after a start near the top of the clock does not give up. Exactly ten seconds still holds.
3. Passing the door clears the search hold before the timeout. The timeout then sets the phase to seeking, so a late clear door ends seeking with the search allowed.
4. The skipped bit is `1 << (cursor as u32 & 31)`. Cursor 32 marks bit 0. Cursor −1 marks bit 31.

## Risks / Trade-offs

- The walker is not yet called from a placed character. The function is the phase's rule.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The door phase was read before this change.
