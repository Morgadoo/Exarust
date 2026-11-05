# Proposal

## Why

Wiring backlog system S5: the content stream's scheduler
(`exa_assets::content_stream`) ran only the level loader's placed objects,
on the main thread. Its object and character rings had no posters, the
stream thread did not exist, the loaded player's request (round 46) was
only logged, and every requested model was built on the main thread where
it was needed.

Phase: Phase 2 streaming and Phase 6 save milestones of docs/PLAN.md (no
PLAN.md edit in this change).

Evidence (REA, Ghidra on `Exanima.exe`): the content-stream unit's set-up
(the ring capacities and the six hooks it installs: job post, object post
and fetch, character post and fetch, reset), its thread loop, the object
builder, the request character builder, the streamed character builder,
the character wait, the object fetch and the character fetch; the item GUI
unit's icon post and icon fetch (the only object poster); the save unit's
identity-reset post and import fetch; the game main unit's named-character
post (`0x500`).

## What Changes

- `exa_assets::content_stream`: each ring step is its own method
  (`take_job`/`finish_job`, `next_object`/`finish_object`,
  `next_character`/`finish_character`, `next_streamed`/`finish_streamed`);
  a request stays at its ring's front while it is built and leaves it after
  its result is pushed; the requested-object missing counter; `ATTACH` and
  `FREE_REQUEST` flags; `step` is built from the steps.
- `exa_assets::stream_worker` (new): the stream thread on Bevy's async
  compute pool (a plain thread without one), passes run one at a time in
  posting order, the rings locked only around their steps, the thread's
  event as a run/signal pair.
- `exa_assets::model`: `ModelJob::from_game` (a job any thread can make)
  and `ModelJob::key`.
- `exa_game::content_requests` (new): the game's stream and builders (the
  object builder loads a record's model from the object libraries for a
  streamed request; the character builder builds a prepared body job), the
  fetches, and the `--sync-content` switch.
- Item icons post their object request and wait on the fetch; the story
  load posts the loaded player's character request with its morphed body,
  and the level's player spawn fetches it.

## Impact

Crates: exa_assets, exa_game, exarust. Specs: content-streaming.
