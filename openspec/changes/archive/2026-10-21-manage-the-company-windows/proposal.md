# Proposal

## Why

The staff, the hire list, the arsenal shelves and the match flow are in,
but the windows the player runs the company through are not: the Manage
Company window's rows and buttons, the arsenal form a fighter or a
candidate is looked at in (its pages, selection, dismissal, rank slider and
points line), Company Status and Company Records. Neither are the company
rules those windows drive: dragging a fighter to another place, an item
sold from the arsenal leaving the company, the day's lessons, and the start
of a tournament event (its bracket and the events the company skipped).

## What Changes

- Manage Company: the row label (profession, or rank and the ready mark),
  the name ink by role, Recruit closed at eight fighters and Hire at ten
  employees, and what a click on a report line opens.
- The arsenal form: its four pages and their shelf filter, the rows of the
  roster and of the hire list, Previous / Next / Dismiss enabled by the
  selection, a step loading its record and committed later, a confirmed
  dismissal moving the selection up, the rank slider's rounding, the points
  line, a record's title, profession line and text, and the skills panel.
- Company Status (staff, masters) and the dismiss and forfeit prompts.
- The roster drag: the new place, the rows that close up and the ids
  remapped in the log's saves, the tournament entries and the order list.
- A sale from the arsenal: a unique item leaves everything; another loses
  one copy (a flagged copy first) and the holders keep what is left, their
  lists keeping their length, their loadouts losing the item.
- The day's lessons: the trainer teaches the manager, then the fighters.
- A tournament event's start: rival winners for the events skipped, the
  extra rounds and the bracket's teams; forfeiting the tourney; arriving at
  the hub; the automatic rematch's lineup.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: manage window, arsenal form, company status, roster
  drag, arsenal sale, lessons, tournament start, hub arrival, rematch.

## Impact

- New `exa_game::{manage, tournament}`; `staff::Staff::{move_employee,
  lesson_round}`; `arsenal::Arsenal::give_up` (uses
  `exa_character::record::CharacterData`).
- Docs: `crates/exa_game/README.md`, `docs/UI.md` (arena windows).
- Evidence: REA/Ghidra on `Exanima.exe` 0.9.5 (the manage window and its
  refresh, the arsenal form's routines and control table, the dismiss and
  yes/no dialogs, the status and records windows, the report click, the
  roster drag, the sale hook, the lesson pass, the tournament start, its
  bracket fill and unentered events, the tourney forfeit, the hub arrival
  and the rematch timer). Notes: `context/re/arena-manage.md`.
- Out of scope: drawing the windows (the GUI skin and form widgets are
  other components); the loadout cost and the outfit copy (character
  record routines); the arsenal view's sorting; the event lineup page's
  weapon write-back; placing or removing figures in the hub; wiring the
  windows into the running game (no arena hub screen runs yet).

## Phase

Phase 7, M7 "arena season": running the company between matches.
