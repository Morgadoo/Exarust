# Design

## Context

Sleep already lies dormant. The contact pass wakes that body when the flags are exactly the dormant flag and a contact was seen.

## Goals / Non-Goals

**Goals:**

- Leave any other flag word, and a sleeper that saw nothing, unchanged.
- Remove role 44 and knock the body out at the current clock.

**Non-Goals:**

- The tone rise that follows the knock-out.
- Choosing the contact. Whether one was seen is an input.
- The path search.

## Decisions

1. The flag test is equality with the dormant flag, not a bit test. A stagger bit kept from before sleep blocks the wake.
2. The knock-out delay is 0, so the wake clock is the current clock. The low flag nibble is cleared and the dormant and waking flags are set.
3. Tone is 0 and the pin gain is the body's 0.08. The countdown is raised to 256 through the body's own wake, so a countdown already at 256 stays.

## Risks / Trade-offs

- The sight check is not yet called from the contact pass.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The sight check and the knock-out were read before this change.
