## Context

The original's spectate is a game mode with its own hooks; Exarust keeps
the session as a frame-side resource the chase camera and the behaviour
tick read.

## Decisions

- The session resource is lent to the simulation bridge (the behaviour
  tick reads it); it holds no simulation data, so no FixedCommand is needed.
- The double click uses the controls' 500 ms, 2-pixel rule.

## Open questions

- Which arena entries reach an employee entrant in the shipped career (the
  four callers): answered by porting XA_Arena's entrant selection.

## References

docs/re/playerctl-map.md.
