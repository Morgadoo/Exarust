# Tasks

- [x] Read the match start, the unrepeated draw, the arena loader, the match tick, the round end, the fall and dead test, the health write-back and read, continue, forfeit and the return to the hub; record them in `context/re/arena-match.md`.
- [x] Add `exa_game::arena_match` with the arena table, the arena and track draws, the outdoor bed, the daily limit, the rounds, the tick count, the falls, the health word, continue, forfeit, the return and the company records, with tests pinned to drawn numbers.
- [x] Add `exa_game::hirelings` (candidate, list renewal, trainer snap, ranked loadouts) and `exa_game::arsenal` (shelves, stow, release), with tests.
- [x] Run the match tick in the game (`ArenaMatchPlugin`) and draw the arena track in play mode at the track's volume (`exa_audio::LevelMusicVolume`).
- [x] Check the eleven track names and the Beast's against the installed `Sound.rpk` (skipped without `EXANIMA_DIR`; checked by hand, no corpus field changes).
- [x] Update `docs/formats/OTHER.md` and the `exa_game` README. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` are the integrator's.
- [x] `make check` (remote: `cargo test -p exa_game -p exa_audio -p exarust`, clippy `-D warnings`, `cargo fmt`).
