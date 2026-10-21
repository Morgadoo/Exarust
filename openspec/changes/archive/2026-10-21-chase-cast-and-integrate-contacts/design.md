# Design

## Approach

Each routine is a pure function over plain state (`exa_ai::cast`,
`exa_ai::chase`, `exa_ai::route`, `exa_ai::contacts`,
`exa_ai::combat::drift_morale`), tested with hand-worked numbers. The
chase calls `exa_world::nav::find_path` through a `Router`, so the grid's
reviewed planner is the only pathfinder. The combat state asks its
`Field` to run the chase; the duellist's field holds the chase state, the
shared plan turn and the grid, and reports the outcome back.

## Decisions

- The chase returns whether the combat tick ends: only while following a
  route. With no route yet, or after leaving combat, the tick runs on, as
  the original's return value decides.
- The duellists share one plan turn a frame (a system-local turn): the
  original's budget of one route plan a frame.
- The place scorer's notice and band terms are not kept: the instructions
  overwrite them with the close formula in the same routine.
- The brain's per-frame target pass is not scheduled: its inputs are not
  produced by Exarust's perception yet, and its leave would fight the
  mind's engage.

## Reverse-engineering questions

- What the cast chooser's busy bit and the chase goal's motion word mean:
  labelled guesses; answered when the power system and the locomotion
  routine are ported (REA, the thaumaturgy unit and the gait routine).
- The integration's calming word and stamp word: labelled guesses; to be
  read from the perception routine that writes them (REA).

## Docs

`docs/NAVIGATION.md` (the chase's planner use) and `crates/exa_ai/README.md`.
