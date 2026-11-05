# Tasks

- [x] Bisect the black creator between the 1 November binary and main in the
  grid. Re-run the 1 November commit and the old binary itself.
- [x] Read the creator's logs: the player is lost after the creator's
  teardown, hears the pit, and the death fade covers the window.
- [x] Take the player's body out of the scene over a plain teardown, and
  bring it back when a level is in.
- [x] Make the pad's `Combat` bit the combat-mode toggle. Add the emulated
  left button as the pad's attack.
- [x] Route `--screenshot` and `--searcher-shots` through the offscreen
  image. Take the search's shots one a frame.
- [x] Tests: the teardown hold and its return, a fallen body left out, the
  Combat bit and the emulated button, the scripted `attack`, the offscreen
  shot target.
- [x] Update `docs/PLAYER.md` and the crate READMEs.
- [x] GPU check in the grid: the creator drawn, the pad's toggle in the log,
  the searcher's shots written from a quadrant.
- [x] fmt, clippy `-D warnings`, tests (exa_game, exarust).
