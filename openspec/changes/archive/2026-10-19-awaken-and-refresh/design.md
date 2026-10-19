# Design

## Context

The wake tick already drops the dormant flag after 1500 ms and finishes the clip base after 8 s. Those are the two calls this slice fills in.

## Goals / Non-Goals

**Goals:**

- Count the persons who can see the body on the tick that drops the dormant flag.
- Refresh reach and thrust preference when the clip base reaches 1.

**Non-Goals:**

- Updating a witness's topic memory.
- The grip pass and the arm-mass scale inside the refresh.

## Decisions

1. A witness counts only when it is a person and it can see. The count is zero on every other tick.
2. Reach is the greatest held reach. When neither hand bit is set, the race reach replaces it.
3. Thrust preference is 0 with no weapon and 10 when slash, crush and pierce sum to 0. Otherwise it is round(thrust / that sum × 5) + 1, and 9 when that is 10 or more. A negative grip mark raises a result below 1 to 1.

## Risks / Trade-offs

- The notice is a count, not a script run. The memory walk is not in.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The event and the refresh were read before this change.
