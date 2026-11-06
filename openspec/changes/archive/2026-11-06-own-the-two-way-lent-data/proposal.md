## Why

A free-running simulation cannot lend the frame's data for each step. After the one-way resources, 35 lent types are written by both the frame and the fixed step; each needs one owner and a path for the other side's writes.

## What Changes

- `exa_physics::sim_app::FrameCommands`: the simulation's queue of writes to data the frame owns (written locally at once, run on the frame right after the step); `sim_app::write_resource` for the frame's writes to resources the simulation owns.
- Frame-owned, copied in, step writes queued: Flow, ItemCursor, DollItems (Arc'd sets), StoryExperience, PlaySession, DeathFade, ChaseCamera.
- Simulation-owned snapshots, frame writes posted: AttackButton, SimControl (frame state split into `SimStepping`), BloodSprays (jets started in the post-step), ClothDebug.
- Moved to the simulation, frame writes posted: WeaponBlood, RoomCells, PassTurn, NavRevision, SearchWorld (`SearchShot` messages out), ScanBuild (debug scan posted), SolverStages.

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the simulation thread.

Rule change: blood jets start in the post-step of the blow (before: at the frame after). Lockstep runs are otherwise unchanged.

## Out of scope

- 17 lent types still written on both sides: Physics, StandIns, ColliderNodes, NavGrid, SoundBank, AudioEnvironment, Assets<AudioSource>, Conversations, ContactMirror, EffectDraws, EffectRoll, 7 gizmo storages; the free loop.
