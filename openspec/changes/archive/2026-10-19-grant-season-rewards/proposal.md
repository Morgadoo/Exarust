# Proposal

## Why

Phase 7 grants four rewards when a season is swept, but it does not say which items. Each season has four item ids, and the fifth season repeats the first.

## What Changes

- A swept season grants that season's four item ids.
- Season 0 is 1175, 1174, 1177, 1176. Season 1 is 1167, 1165, 1168, 1169. Season 2 is 1171, 1170, 1173, 1172. Season 3 is 1178, 1179, 1180, 1181.
- The season is the day divided by 28, wrapped to four. Day 134 grants season 0 again.
- A season the company did not sweep grants nothing.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the four item ids a swept season grants.

## Impact

- `exa_game::career`, the night that already closes a season.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: four rows of four item ids, chosen by the day divided by 28 and wrapped to four, granted only when every week of that season was won. No format field changes, so the corpus step is a skip.
- Out of scope: emblems, placing the item in the arsenal, and a candidate's listed price.
