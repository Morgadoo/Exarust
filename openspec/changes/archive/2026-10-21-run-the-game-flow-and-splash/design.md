# Design

Rules page: [`docs/UI.md`](../../../docs/UI.md), The game flow and The
splash screen. No `docs/formats/` page changes: the flow reads no new
format.

## Steps instead of calls

The original's routines write globals and call the rest of the game. The
flow returns `Step`s (show a page, load a level, save, place the player,
frame the camera) so the order is testable and each host can apply the
steps it owns. Nothing applies the world steps yet; `FlowPlugin` logs them.

## Faithful quirks kept

- The exit-trigger nudge's negative branch moves `extent + |local| + 75`
  toward the negative side, and its box check compares the squared
  overhang with the plain 50.
- The practice next fight computes a +5/+13 cm height and overwrites it
  with the body's stand height before placing the body.
- The splash's elapsed times are signed differences of the frame time and
  the stamp clock; a stamp taken after the frame started gives a negative
  fade whose byte conversion draws one frame white.
- A paused quit runs the arena unit's pause callback in any mode.

## Open questions

- The per-mode flag words the switch writes (read as checkpoint use and the
  arena's inventory override): their readers are outside the unit; answered
  by REA on the readers when the save and inventory flows are wired.
- The story start's spawn height reads a character-record float (×5 + 25);
  its meaning is unnamed. REA on the character record's writers.
- The two random-number calls take the process generator; the callers pass
  draws until the run-time library's generator is shared.
