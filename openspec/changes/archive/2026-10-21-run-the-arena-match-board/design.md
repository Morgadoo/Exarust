# Design

## Behaviour

The board keeps one selected match record. The standard tab copies one of
six standard matches and writes the rank into the floor and rank fields and
the tier (none for Pugilism) into the highest-rank and tier fields; the fee
and prize are unpacked to copper (twelve to the silver, twelve silver to
the gold), multiplied by the rank plus one in 32 bits, and repacked with a
16-bit gold count and wrapping silver and copper bytes. The events tab
copies the selected daily event, skipping greyed ones; the tournament tab
copies the selected tournament event. The rebuild walks the lineup order and
adds each combatant whose rank passes the floor, stops at the side size and
admits one leader, the player included, unless flag 1 is set. The four move
paths follow the original's two drag handlers and two double-click
handlers; the lineup-order target stays at the first place when the
original leaves it unset (a marked swap or a bench).

## Model

`Board` is plain data with methods; `Board::paint` returns a draw list built
from the same rectangles as `Board::hit_test`. The primary button's state is
kept in its control word and updated after each rebuild and move, as the
original's label routine does. The weapon check compares the first entrant's
item kind (supplied by the caller from the fighter's event value) with the
tier's weapon row, stopping at the first zero variant.

## Open questions

- The form template's control rectangles and labels are loaded outside the
  board's methods: placed from the template's order and labelled guesses.
  Answer with a capture of the running board.
- The row height (30 pixels) fits six rows in 180 and eight in 244: a
  guess until a capture.
- Flag 1 on a daily event greys the row; what sets it (likely an entered
  event) is outside the board. Answer with REA on the arena's entry code.
- The day-end role of control 12 is inferred from its enable rule; its two
  tournament-day checks are the arena's. Answer with REA on those checks.
- The item kind behind a fighter's event value comes from an item lookup
  outside the board.

## Docs

`docs/UI.md` (Arena match board) describes the rules; there is no
`docs/formats/` page for this change.
