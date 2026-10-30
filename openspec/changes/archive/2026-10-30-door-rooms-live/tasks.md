# Tasks

- [x] Trace the door binding's callers and body lookup, the passage flag, the room manager's readers and the portal pass (REA).
- [x] Every door neighbour of an active room is adjacent; shut links in a hash set.
- [x] `link_open` and `RoomTracker::set_door`.
- [x] Bind placed doors at spawn; follow their bodies each frame.
- [x] `--toggle-door` debug flag; active-room debug log.
- [x] Tests: the door's 0.4 rad and 0.8 travel numbers, locked doors, shut doors keep rooms adjacent, the installed exanima01's doors bind (skipped without the game).
- [x] Docs: RENDERING.md, exa_world README.
- [x] GPU checks: exanima06 golem gate, exanima01 door toggled open, exanima06 open doorway.
- [x] fmt, clippy and tests for `exa_world`, `exa_ai`, `exarust` on the worker.
