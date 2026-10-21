# Design

Rules: [`docs/THAUMATURGY.md`](../../../docs/THAUMATURGY.md), In the game.

## Decisions

- One `PowerCaster` component per casting body holds the cast body, the
  record, the bar and the active list. The bar's cast entry
  (`TreeScreen::cast_slot`) is called as it stands; the domain front ends
  run inside it, and the creator runs after a start.
- The active list is walked in the body tick with event 0. A power told to
  end (state ending) gets event 2. The body's release flag is not mapped to
  an event: no caller of the original does.
- The caster and the screen share one record. The screen owns it while the
  tree is open; the caster takes it back when the tree closes.
- The cursor point is the aim plane's point, taken as exact (inferred): the
  original's hit and back-off along a surface normal need a surface hit.

## Not wired

Listed in the proposal's out-of-scope paragraph; the debug lines of the
creators name each missing piece.
