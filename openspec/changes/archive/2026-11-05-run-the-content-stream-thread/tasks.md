# Tasks

- [x] Trace the unit's set-up and hooks, the thread loop, the object, request-character and streamed-character builders, the fetches and waits, and the posters (item GUI icons, story load, game main) in REA.
- [x] `exa_assets::content_stream`: ring steps as methods, request kept queued during its build, the missing counter, `ATTACH`/`FREE_REQUEST`; tests.
- [x] `exa_assets::stream_worker`: the thread on the async compute pool; posting-order, pending-during-build, missing, restart and terminate tests.
- [x] `exa_assets::model`: `ModelJob::from_game`, `ModelJob::key`.
- [x] `exa_game::content_requests`: the game's stream and builders; install-gated tests over a shipped model.
- [x] Item icons through the object ring; the loaded player through the character ring and the spawn's fetch; `--sync-content`.
- [x] Crate READMEs.
- [x] GPU check: a story save's load, the icons, timings with and without the thread.
- [x] fmt, clippy -D warnings and tests for the touched crates on the worker.
