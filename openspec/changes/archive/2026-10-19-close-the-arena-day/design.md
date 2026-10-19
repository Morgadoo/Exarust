# Design

## Context

A new career starts on day 0. Employees have a profession in the top of the record word, and six skill levels stored as multiples of 5000.

## Goals / Non-Goals

**Goals:**

- Price a day's wages and move the day counter.
- Resolve a week of six winners and a four-week season.
- Drop an entry under health 3276 at the end of a week.

**Non-Goals:**

- The hire price, fee scaling, healing, restock, and which items a season grants.

## Decisions

1. Skill levels are the stored words divided by 5000. The wage uses at most six of them.
2. The season is four weeks because the verdict stores four results and grants the season when the week index is 3 and all four are the company's.
3. A tie, or fewer than three wins, does not take the week. The company's id in the results is 1.
4. The purse does not go below 0. The bill is still added to the lifetime spent.

## Risks / Trade-offs

- The save does not yet write the day, the weeks, or the skill levels. A loaded career is back on day 0.

## Migration Plan

No data migration. Existing saves keep their previous fields.

## Open Questions

- The numeric season-length word was not read from the image. The four stored weeks and the week-index check of 3 are what this slice follows.
