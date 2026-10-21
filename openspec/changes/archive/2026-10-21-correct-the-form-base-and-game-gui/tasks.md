# Tasks

- [x] Re-read the form class's and the in-game unit's instruction listings;
      record every discrepancy with its evidence.
- [x] Correct the form base (layers, raises, active test, places, maximise,
      restore, resize, fade, scrollbars, wheel, drags, points, hit test,
      hover, focus, caret, Tab, routes, press and release codes, snap slots,
      rescale) with tests pinned to worked numbers.
- [x] Correct the in-game unit (tags, sizes, places, show rules, journal,
      power slot, counter, marker, alert, curtain, compass, heartbeat fade,
      menu cursor, draw order, scopes, selector rows) with tests; remove the
      copies of `hud` and the placeholders.
- [x] Keep the HUD bars' compared values to the frames that show them.
- [x] Send the I and J hotkeys' screen requests through the hotkey handler;
      open the inventory and journal on the window manager; `--press Esc`.
- [x] GPU smoke on this PC: I, J and Esc in `play exanima01`, frame time
      against main.
- [x] Update `docs/UI.md` and the READMEs. `docs/PLAN.md` and
      `docs/COMPONENT-MAP.md` are the integrator's.
- [x] No corpus format changes; nothing new reads `EXANIMA_DIR`.
- [x] Tests and clippy (`-D warnings`) for `exa_ui` and `exa_game`.
