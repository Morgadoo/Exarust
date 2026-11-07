## Decisions

- The snap runs where the scene command builds the body (the frame world,
  against the collision replica), so every arena path that plans a snapped
  body gets it without each caller casting.
- A miss keeps the planned place (as the original does for the snap); for
  the load placement the original would keep the probe's lifted start, which
  Exarust does not reproduce.
