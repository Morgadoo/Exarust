# Proposal

## Why

The fifth pass on the content stream and texture units (lane `streaming5`,
docs/PLAN.md Phase 2, Static world: texture loading and the content stream)
wires the two rules the fourth pass left without a caller: the terrain
material path's stream-first texture load and the content stream's level
pass over a level's placed objects. It also adds the texture manager's
fast-load flag and the level wait's report and handoff.

Evidence: the original's instructions, read through REA and the
disassembly (the terrain path's status word and its missing-provider
return, the material path's fast-load branch and the flag's single read,
the level wait's message, its room-mode test and the scene constructor's
room-mode word); the real installation for the level-load checks.

## What Changes

- texture-decoding: new Terrain texture path and Fast-load flag.
- content-streaming: new Placed objects through the stream and Level wait
  report and handoff.
- Code: `ModelLoader::texture_waiting` and `texture_finish_now`;
  `TextureCache::fast_load`; the terrain build queues and waits for its
  palette textures; `LevelLoad` drives `ContentStream` for placed objects;
  `content_stream::{missing_message, LOG_SOURCE, LevelHandoff}`;
  `LevelInfo::{missing_database_objects, player_focus}`.

## Out of scope

The request rings and streamed characters still have no driver (the
character spawner and database object requests build directly); the
session's arrival words are not yet passed from the game to `LevelLoad`;
the grid environment's room sorting is Exarust's existing per-object room
assignment, not a handoff of the list.
