# Tasks

- [x] Buffer cache, sets and picker in `exa_audio::bank`, with tests of the generator words, picks, WAV walk, durations and lengths.
- [x] Voice mixer in `exa_audio::environment`: allocation, scan, audibility, sort, binding, refresh, eviction, pause, handles, focus and buses, listener, distance model.
- [x] Music crossfader, Music setting and the level change in `exa_audio::music`.
- [x] Area ambience in `exa_audio::ambience`; reverb block and decay in `exa_audio::reverb`.
- [x] Playback layer: spatial and plain sinks, the distance model, the restart rule, the listener anchor on the player.
- [x] Verify against the installed `Sound.rpk` (the installed-sets test, skipped when `EXANIMA_DIR` is unset).
- [x] Update the crate README; `docs/PLAN.md` and the component map are left to the integration lane.
- [x] `make check` equivalent: fmt, clippy -D warnings and tests for `exa_audio`, `exa_game` and `exarust`.
