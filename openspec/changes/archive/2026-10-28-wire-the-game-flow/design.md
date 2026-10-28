# Design

- One queue, `FlowSteps`: the boot, the frame tick, the player follow, the
  teleporter exit, the menu choices, the splash and the end menu push their
  steps; `apply_steps` runs last in the flow's chain and applies only what
  the app owns. Everything else is logged, so the rules stay the single
  source of the order.
- `BootToMenu(false)` from the launcher keeps the boot title off a direct
  `play` run; the splash still shows the title at its hand-over, and a
  play level's player keeps a title that is up instead of hiding it.
- A launched level is a practice-mode session without the round hook: the
  round rule counts any dead character, and a story level holds corpses.
  A practice started from the menu installs the hook as the original does.
- The menu text program is a draw-path mode (`SkinMode::MenuText`) so the
  CPU rasteriser runs the same fragment rule the unit's shader does; the
  menu's own `menu_text::texel` delegates to it.
- The end menu's placement (centred rows under a dark backdrop) is
  Exarust's: the original's form class is outside the game main unit. The
  rows, order, scales and dispatch are the original's.
- The frame cost with no menu open is unchanged: the draw systems return at
  once while their forms are closed and the GUI canvas hides when no frame
  was queued.
