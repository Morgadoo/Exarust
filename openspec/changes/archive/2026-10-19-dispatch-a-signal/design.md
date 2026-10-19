# Design

## Context

Levers and plates already decide a value. The dispatch is what that value does to the named target.

## Goals / Non-Goals

**Goals:**

- Choose close, open or toggle for a door, including the inverted flag.
- Start a fire or set a light bit. Name a relay kind. Push a plate.

**Non-Goals:**

- The relay bodies, and carrying out the close.

## Decisions

1. The lookup fails closed: a target that was not found does nothing, even if the other fields are set.
2. A door record is handled before the operative class. −1 is the toggle and is not passed through the invert. Invert keeps only the flipped low bit, so 0 and 1 swap and any other value becomes 0 or 1.
3. Toggle is per leaf: unlatched closes, latched opens.
4. A fire uses the stored size and does not change it. A light writes bit 0 of that word. Relay kinds outside 0, 1, 2, 3 and 5 are ignored.

## Risks / Trade-offs

- Nothing in the world calls this yet. The function is the dispatch's rule.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The dispatch, the job and the toggle were read before this change.
