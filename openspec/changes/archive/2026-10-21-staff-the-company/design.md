# Design

## Context

`exa_game::career::Company` keeps a simplified roster for the session save.
The original keeps a list of 56-byte employee records (a class byte, the
character, a personal item list) and renumbers its fighters' ids after every
change; the hire list, the arsenal and the tournament entries refer to
those ids. The daily report reads the log, the fighters' skills and the hire
list.

Rule page: [`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Arena
tables).

## Goals / Non-Goals

**Goals:** the staff as the original keeps it, with every check, message,
count and side effect of a hire, a recruitment, a leaving and a dismissal,
and the report's lines, all as plain functions with tests.

**Non-Goals:** the windows, figures in the hub, world characters, save
chunks. `career::Company` is left as it is; the staff is its own type.

## Decisions

1. The candidate's character is reduced to what the routines read: its role,
   name, own items and six loadouts (`staff::Hire`).
2. The arsenal lookups take an item-kind closure, as `arsenal` does.
3. The order list keeps the original's removal: one entry shorter every
   time, the last one dropped when nothing matches, later numbers left as
   they are, a doubled match passed over.
4. A name is the record's sixteen bytes up to the first NUL.
5. "Ready to train" is the original's test: no skill below level 5 with its
   next technique chosen, and a level sum under 15.

## Open questions

- What the report codes beyond the line kinds are used for (a double click
  acts on a line): answered when the manage window is built, by reading its
  click routine with REA.

## Risks

- Two rosters exist (`career::Company` and `staff::Staff`); a later change
  that builds the manage window should fold the session save onto the staff.
