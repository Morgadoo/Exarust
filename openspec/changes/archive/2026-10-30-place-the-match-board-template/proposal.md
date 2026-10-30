# Proposal

## Why

docs/PLAN.md Phase 7 (game modes and content: the arena career) needs the
arena "Matches" board to look and behave like the original. The board's
own controls (tabs, bottom row, captions, season picture and item slots)
come from a static form template that Exarust only guessed at, the day-end
button relied on a caller-supplied verdict, the board's End Day, Close and
Forfeit buttons did nothing, and a reopened board forgot its tab.

## What Changes

- The form template's 23 controls: captions (Standard, Specials,
  Tournament, Season, Entry:, Prize:, Available, Entered n/m, Prize,
  End Day, Close, Enter, Forfeit, Prize for this season), initial states and
  rectangles placed by the toolkit's placement rule: x snapped to a
  multiple of four with ties to even, y in four-pixel rows lifted by half
  the height less eight up to 32 pixels and by four above, bottom-anchored
  controls `y + 8` rows above the parent's bottom edge.
- The day-end control computed from the entries: on the tournament day with
  an entrant it is enabled once every entered event has a winner; otherwise
  once a match has been played.
- End Day, Close and Forfeit are hit-tested by their state and reported as
  board actions; the hub's form host holds the bottom row.
- The coin columns carry their callback's decision (dash four pixels in, or
  the coin counts) in the paint list.
- Closing saves the tab and three list selections and the next opening
  restores them.
- Review corrections: the show's forced tournament switch hides the tab it
  leaves; nobody entered clears the weapon mark; the tournament winners
  read the company table at entry `winner - 2`.

## Evidence

The board's constructor, show, tab switch, weapon check and winner table
fill, the arena's day-end condition, the form template data (23 records of
104 bytes) and the toolkit's template placement routine, read against
their assembly in `Exanima.exe`.

## Out of scope

The item GUI and texture manager calls (season and tournament item
objects, picture loading, coin icons), the class tables' default label and
item-slot sizes (guessed as 16 and 60), the parent's bottom-edge field
(taken as the form height), and the arena's handling of End Day and
Forfeit (the board only reports them).
