# Proposal

## Why

docs/PLAN.md, Phase 7 (game modes and content), arena paragraphs: the live
hub kept only seven of the company's thirteen records, its End Day skipped
the manager's rest and lesson, the hire list, the boards' new rolls and the
restock, and the hire page listed nobody. This change carries a DeepSeek
pass on the arena unit and its review.

## What Changes

- `career::Company` keeps all thirteen records (`records`/`book_records`)
  and splits the day's earned and spent from the lifetime totals the night
  rolls them into; the day's spent then holds only the wages.
- The match start counts a Beast match entered; the return books wins,
  losses, forfeits, Beast wins and the company deaths the match's falls
  counted onto the career.
- End Day rests the manager and gives him the first trainer's lesson,
  books a tournament lost when the company entered a week a rival took,
  then asks the restock, renews the hire list, rolls a new week's board the
  day after a tournament day and rolls the day's matches.
- The hire list is live: ten candidates on a new career, the hire page's
  fee and upkeep, HIRE through the company's checks (one more hire and the
  joining log), the dismissal books one more fired and its log.
- Review fixes: candidate levels and board rank rolls read the run-time
  library's Mersenne twister (one sequence on the career), not LCG copies;
  candidates carry their six levels so the upkeep reads them; a new week's
  board clears the entries and becomes the career's rival field instead of a
  second field; the arena reset keeps the hire list and the week's events
  and rolls the day's matches; a new career rolls the tournament before the
  hire list; the save's coin records and day tallies are coin words, the
  day tallies and the prize history come from the career; the match's
  company deaths reach the career.

## Evidence

The retail assembly of the day-end routine (the day count, the manager's
and staff's rest, the lessons, the two lifetime adds through the coin adder,
the wage sum set through the coin setter, the purse's clamped subtraction,
the shop and merchant restock, the hire renewal, the week verdict and new
board on day mod 7 of 1, the daily board), the week verdict (the tournament
lost record raised when a rival or no one takes an entered week), the hire
routine (the hires record and the joining log), the dismissal commit (the
fired record and its log), the match start (Beast template 11), the return,
the fall routine (the deaths record), the reset (sixteen record words and
both day tallies cleared, the hire list untouched, the daily board rolled),
the new-career routine's order, the candidate draw (its level from the
library's float random, which reads the Mersenne twister), the coin setter
(gold over 144, silver over 12) and the company and day chunk writers.

## Out of scope

- The candidate's item outfit (no item-fact table in the hub) and its gear
  share of the fee; the kit release a dismissal makes (no live arsenal).
- Saving the week's event rows and the hire list; the session save's own
  company payload keeps only its old fields.
- The under-filled event clearing on a tournament day; the exact counter
  reads behind the restock and candidate seeds.
