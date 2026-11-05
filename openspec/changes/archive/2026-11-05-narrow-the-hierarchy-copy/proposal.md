## Why

Stage 4 of the simulation thread, after `bodies-as-snapshots`. The bridge scanned the hierarchy and transforms of every frame entity each step, though the simulation uses only those of the entities it holds; the frame's meshes, bones, lights and GUI made most of the scan. The hand's cloth grab also read the cloth as it was built.

## What Changes

- On the frame's side an entity is marked `SimShared` once a bridged type other than the hierarchy and transforms crosses for it, either way, with its ancestors. `ChildOf`, `Children`'s order, `Transform`, `GlobalTransform` and `Name` are scanned and sent only for marked entities; a newly marked entity sends them whole.
- A `Children` order applied from the other side moves only the children it names.
- `EXA_BRIDGE_WIDE=1` turns the narrowing off (a debug aid).
- Behind the bridge the frame's cloth particles take the newest mesh places, without a change mark, so the hand's grab picks the vertex where it hangs now.

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the simulation thread.

Evidence: a copy-bridge matter, no original routine involved. No rule changes.

## Out of scope

- Stage 6: the simulation running free of the lockstep.
