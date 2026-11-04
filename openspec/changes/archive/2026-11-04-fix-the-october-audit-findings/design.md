# Design

Formats page: none (no format change); behaviour pages `docs/UI.md`
(Death fade) and `docs/formats/RSQ.md` (Falling to death).

## The fade's clock

The original stamps its frame clock at the pit and completes the fade when
the frame clock passes the stamp by 3000 ms; its simulation ticks are
scheduled against the same clock (12 ms a tick), so the death tick (3000
ms after the pit tick) has run when the fade completes. Exarust's virtual
clock advances 1/83 s a fixed step while the game clock counts 12 ms, so a
fade on virtual time completed about 12 ms before the death tick and paused
the game first. The fade now reads the game clock between ticks: the last
tick's own clock plus the fixed step's gathered share of the next, and
starts at the pit tick's own clock.

## The player's body

The original removes the player's node from the scene but keeps it as the
session's player; Exarust despawned the root, so the story hook read no
player and never opened the end menu. The body now gets `OutOfScene` and is
hidden with its root.

## Open questions

- Which of the leave routine's first calls are Exarust's session task and
  game view steps: not traced; the reset is placed after them (a guess
  noted in the code).
- The two-hander grip's head-height mark: the armour pass was not found
  cheaply; left as a guess.
