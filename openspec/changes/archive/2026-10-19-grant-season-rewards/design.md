# Design

## Context

Ending the fourth week of a swept season already adds four to the reward count. The ids themselves are four rows of four. The row is the day, after it has moved on, divided by 28, wrapped to four.

## Goals / Non-Goals

**Goals:**

- Record the four ids when a season is swept.
- Repeat the first row on day 134.
- Grant nothing when the season was not swept.

**Non-Goals:**

- Creating the item, or the three bytes written on the new record.
- Emblems and a candidate's listed price.
- The Exarust save. It already leaves the season count at 0 on load.

## Decisions

1. The ids are stored in the order the season grants them, four at a time.
2. The row uses the same day the week index already uses, the day after it has moved on.
3. A season is swept only when this week is ours and every earlier week of the season was ours. That is the check the night already makes.

## Risks / Trade-offs

- The save still does not keep the season. A loaded career has an empty reward list, as it already has a zero reward count.

## Migration Plan

No data migration. No format page: no field changes.

## Open Questions

- None for this slice. The sixteen ids and the wrap at four seasons were read before this change.
