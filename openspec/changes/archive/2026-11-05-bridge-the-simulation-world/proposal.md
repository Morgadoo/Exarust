## Why

Stage 3 of moving the fixed step onto its own thread (`plan-the-simulation-thread`, `shell-the-simulation-world`). The boundary map shows that the core simulation state (`Ragdoll`: 59 fixed-step writers, 38 readers, 22 frame writers) is shared by almost the whole fixed step, so moving one data type at a time would drag the whole step with it. The fixed step's schedules therefore move whole into the simulation's world, still in lockstep, and the data they touch is bridged by copying until later stages turn the frame's writers into commands and the copies out into snapshots.

## What Changes

- `exa_physics::sim_bridge`: `SimBridgePlugin` moves the fixed step's schedules (`FixedFirst` to `FixedLast` and the ones it names, such as the character tick and the brain chain) with their sets and order from the frame's world into the simulation's, on the first lockstep step.
- A lockstep copy bridge: before each step the frame's changes since the last step (changed values, removals, despawns, messages) are applied to the simulation's world; after it, the simulation's changes go back. Each side's own writes from an apply are not sent back, so change detection (`Changed`, `Added`, `RemovedComponents`) sees on each side what it would see in one world.
- Registration per plugin (`SimBridgeAppExt`): copy (clone when changed), copy of an immutable relationship, lend (the value moves across for the step and back, for large or uncloneable data), the same for resources, message forwarding both ways, and sim-only data.
- Entity ids are the same in both worlds: the frame's allocator hands out every id, the simulation spawns its own entities from ids the frame reserves for it, and despawns are mirrored without freeing the id.
- Coverage: when the bridge starts it builds the moved schedules and lists every type their systems touch that is not bridged; `--sim-boundary` adds the same list ("Bridge coverage"). Cost: `BridgeCost` counts values and copy time per half, logged every 1000 steps.
- Moved behind the bridge with full coverage in their tests: the bodies (`BodyPlugin`: bodies, collision world and solver stages lent, the level's collision grid copied), and the character tick with the collapse's posture branch (`CharacterTickPlugin`, `CollapsePlugin`). Bevy's hierarchy, transforms, names and clocks are bridged by the plugin.
- `--sim-bridge` runs the game's fixed step behind the bridge (opt-in: the rest of the fixed step's types are not registered yet).

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the fixed tick's cost.

Evidence: the original's physics thread body read with REA for `plan-the-simulation-thread` (the simulation runs on its own thread and owns its objects; the window side hands work over through the deferred-call ring). This change adds no engine rule: parity is shown against Exarust's own one-world fixed step (bit-for-bit body poses, the same stage results, the same change-detection log).

## Out of scope

- Registering the rest of the fixed step's types (thaumaturgy, brains, combat impacts, grips, mechanisms, cloth, props, effects, the player): the coverage list is the remaining work.
- Replacing the frame's writers of simulation data with commands, snapshots in place of copies out (stages 4-6).
- Running the simulation free of the frame; thread priority.
- The character controller's internals (round 63's `cl/charmove` systems move with their sets, unchanged).
