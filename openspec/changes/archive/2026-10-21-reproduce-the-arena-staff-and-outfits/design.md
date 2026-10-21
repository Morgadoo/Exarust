# Design

## Context

The arena routines draw from the engine's linear-congruential generator in a
fixed order and narrow several double-precision sums once. A reordering of
draws or a single-precision shortcut changes outputs, so every rule below was
checked against the instructions rather than the decompiler's text.

## Decisions

- Draw order is the original's, including picks that read the generator
  before testing their guard; a guarded pick and an unguarded pick are
  separate functions.
- Sums the original does in double precision (look sums, the roster health
  fill, the hire fee's fifths) are kept in double and narrowed once.
- The fighter rank, the hire fee and the end-of-day rule exist once
  (`arena_ids::fighter_rank`, `career::hire_fee`, `arena_match::day_can_end`)
  and the manager form calls them instead of carrying copies.
- The loadout form's decisions are pure functions over plain values (page,
  selection, counts) so a window can call them later; no state is held.
- Where the original's choice is a callback or is not read (icon handles,
  the hub's ambience flag), the module says so and assumes no table.

## Risks

- The spawn plans have no caller, so their output is checked only against the
  plan's own tests and the instructions.
- Several documented approximations remain (see the proposal's out of scope).
