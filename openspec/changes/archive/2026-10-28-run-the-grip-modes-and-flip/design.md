# Design

- The grip runtime lives in `exa_character` because it reads the holder's
  `Animator` and `Condition`; it runs before the grip solver's sync set
  (`GripSolveSet::Sync`) so the tick's solver sees its links.
- A held item names its holder by the holder body's first part; the flip
  request names the character entity and is resolved through its body.
- The item's top body moves its root entity, which carries the blade, grip
  and carried fire, so the blade sweep and flame follow the swinging body.
  The flip turns the item through its body (the flip link), not by turning
  the entity by hand.
- The factory body is not attached because the shape builder has no game
  caller yet and its fit has not been checked against a shipped weapon.
- The base sound's stand-in set is the weapon's hit set, whose clock the
  striking branch shares, so a missing set does not park that clock.
