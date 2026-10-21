# Design

## Context

`exa_game::{staff, arsenal, hirelings, report, arena_match}` hold the
company as the original keeps it. The windows over it are TGUIForm
instances whose controls come from tables of 104-byte entries; their
handlers live in the arena unit. Exarust has no arena hub screen yet, so
the windows are modelled as state and plain functions the screen will
call.

Rule page: [`docs/UI.md`](../../../docs/UI.md) (arena windows).

## Goals / Non-Goals

**Goals:** every check, flag, clamp, text and side effect of the
handlers, with tests pinned to numbers read from the instructions.

**Non-Goals:** drawing, widgets, figures, save chunks.

## Decisions

1. The arsenal form is `manage::ArsenalForm`: page, last row, selection,
   pending row, shown and kept tab, the dismissal byte and the three
   buttons' flags. A step returns the record to load; the commit takes a
   closure that rebuilds the rows after a dismissal.
2. Records are named by `RowRecord` (candidate, manager, employee place),
   not by pointers.
3. The rank slider converts in 64 bits and keeps the low half, so a NaN
   names rank 0 (the conversion's 0x8000000000000000).
4. The sale keeps the original's quirk: a holder's list is closed up on a
   copy of its count, so the list keeps its length and its last entry stays.
5. The roster drag's id map covers ids 0 to 10 as the original's does; an
   id past it is left alone (the original reads the stack).
6. An ordinary item with no shelf is left alone by a sale (the original
   indexes outside the shelves).
7. The rematch lineup stops at the end of the staff (the original reads on).

## Open questions

- What the skills panel does with the byte the hire page and a trainer set
  (guess: it fixes the techniques): to be read with the TCompactSkillUI
  routines.
- The hub object a hired physician removes: to be read with the hub layout
  when the hub screen is built.

## Risks

- `career::Company` still teaches only its combatants; the original's
  lesson order (manager first) is `Staff::lesson_round`. Folding the
  session save onto the staff is still open.
