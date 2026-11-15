## Why

Roadmap task B1 (docs/roadmap/phase-b-close.md), the `wirearena` lane: the
arena game mode's family (the match board, the arena unit's routines and the
end-session menu). The modes audits found the board's tournament entries and
the season's week winners never filled by live code, the board's fighter
tables without their double press and drag and drop, the end-session menu's
arena CONTINUE leaving to the title as a stand-in, and the session leave's
arena step only logged. Manage Company's report passed an empty hire list and
no fighter names.

## Evidence

REA (Exanima.exe), pseudo-code checked against the assembly where order and
constants matter:

- The board's constructor callbacks: the company table's double press (rank
  check, then the move into the lineup), the lineup's double press (back to
  the company table), the drag-over rank check, the company table's drop
  (refuses a drop from itself; sorted insert; on the tournament page the
  event's entrant leaves and the later ones move down) and the lineup's drop
  (marked swap, captain rules, then on the tournament page every entrant is
  rewritten: count, id, the fighter's event value into the first word, the
  second word's item cleared; otherwise the fighter moves in the arena's
  order list).
- The lineup rebuild reads the arena's order list (one entry per fighter,
  rebuilt when its length is not the staff's plus one) and, on the
  tournament page, the event's entrants by id.
- The fighter's event value is the first hand item of its loadout at the
  event's place; the weapon mark compares that item's kind with the event's
  weapon row.
- The week verdict writes the week slot: 1 for the company, else the
  winner's field entry plus two (the field holds a company id less one); the
  fourth week clears the four slots.
- The end-session menu's arena item calls the arena unit's continue (the
  next round, the settlement and the way back to the hub, or the death
  screen), the same routine as the arena's own end screen.
- The session leave calls the arena's leave in game mode 8: the hub's busy
  flag set and the click hook cleared (and one more global of untraced role).
- The report's log line names a fighter through the by-id lookup (0 the
  manager); its hire lines count the hire list's professions.

## What Changes

- `ArenaCareer::board_with` fills the board's entries from the career's six
  tournament rows, the fighters' event values from their loadouts, the
  season's week winners, and the lineup order from the staff's order list.
- `HubWindows::board_table_input` and the `board_table_input` system give
  the board's company and lineup tables their double press and drag and
  drop; `board_table_answer` writes the change back (the event row on the
  tournament page, the staff's order list otherwise).
- `Company::week_winners`, written by the week verdict
  (`Company::end_day_in_field`), saved in the tournament chunk's weeks and
  restored by the load.
- The end-session menu's arena CONTINUE sends the arena's continue.
- `ArenaLeave`: the flow's arena leave step makes the hub busy and clears its
  click hook; the hub's clicks answer nothing until the next entry.
- `ArenaCareer::manage` passes the hire list's professions and a by-id name
  lookup to the report.
- `--board-table STEPS` (debug): scripted pointer steps on the board's
  fighter tables.

## Out of scope

- The arena New Career's company-name and emblem dialog before the creator:
  the menu command is answered in the character creator's front end, which
  belongs to the creator's owner.
- A drag inside the lineup (the original's lineup drop also takes its own
  rows and reorders); the board model's drop takes company rows only.
- The board's lineup button (the event lineup in the loadout form's mode 3)
  and its close hook.
- The arena save's id preparation and post-load remap.
