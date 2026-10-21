# Proposal

## Why

The arena's "Matches" board only had its close bookkeeping and destroy in
Exarust. The arena career cannot offer its standard matches, daily events,
weekly tournament or season standings, nor let the player pick a lineup,
without the board's rules: which fighters may join, how the lineup fills,
what each match costs and pays at a rank, and what each tab shows.

## What Changes

- The board copies one match record per tab (a standard match, a daily
  event or a tournament event) and builds the lineup for it: its lowest
  rank is the floor, its side size caps the lineup, the Captain event lets
  teammates be two ranks lower, flag 1 lifts the one-leader and
  marked-fighter rules, and flag 4 names a tournament weapon.
- The six standard matches with their side sizes, fees and prizes; the
  standard tab scales the fee and prize by the rank plus one with the
  original's coin packing, greys the tier rows above the rank and moves the
  tier with a rank change.
- Double click, drag and drop between the company table and the lineup:
  room, bench, swap, marked swap, the Captain checks, the lineup order and
  the slot-ordered company table.
- The daily event, tournament winner and season tables, the tournament
  status line and buttons, the weapon check, the day's message, the
  "Entered n/m" label, the primary and day-end controls, the show, close
  and tab switch, the form layout, paint order, hit test and clicks.
- `exa_game::arena_board` fills a board from a career and the generated
  daily board.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the arena match board's tabs, records, lineup and
  layout.

## Impact

- `exa_ui::match_board` (`Board`, `MatchRecord`, `STANDARD_MATCHES`,
  `WeaponRow`, the coin helpers, layout and paint types) and
  `exa_game::arena_board`.
- `docs/UI.md` (Arena match board), the `exa_ui` and `exa_game` READMEs.
- Evidence: REA/Ghidra on `Exanima.exe` 0.9.5: the board's 30 methods
  (constructor, tab switch, show, close, destroy, standard, events and
  tournament refreshes, the event and winner fill, the tournament status,
  the label routine, the lineup rebuild, the four move paths, the rank
  checks and the coin cell callback), read against their assembly; the
  standard-match and rank-label tables and the weapon tier table read from
  the image; the rank helper, the weapon membership test and the day-end
  rule outside the board. No corpus files.
- Out of scope: the form template's own control rectangles and labels, the
  item-GUI, texture and skin calls behind the item slots, coin icons and
  health cells, the item kind behind a fighter's event value, the arena's
  two tournament-day checks for the day-end control, the bracket and match
  start, and a Bevy plugin that draws the paint list.

## Phase

Phase 7, game modes and content: the arena's match board.
