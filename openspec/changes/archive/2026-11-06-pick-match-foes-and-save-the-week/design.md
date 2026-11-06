## Decisions

- The pick replaces the foe only when it differs from the duellist's own
  (so a duel against the player runs exactly as before; trace-diffed).
- Scene bodies had no relations, so the role database rated everyone 0;
  they now carry empty relations as placed characters do.
- The match grace holds the brain's combat entry [guess: the original's
  AI timers gate the activity that runs the pass].
