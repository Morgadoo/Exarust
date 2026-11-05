## Context

Design: `plan-the-simulation-thread` (option A, a simulation sub-app with its own world). Shell: `shell-the-simulation-world` (`SimShell`, inbox, outbox, snapshots, `SimId`, lockstep before `FixedFirst`). No `docs/formats/` page is involved: no data format is read.

## Decisions

### Move the schedules whole
The first lockstep step removes the fixed step's schedules from the frame's `Schedules` (the labels of `FixedMainScheduleOrder`, read in `First` before any fixed step since `FixedMain` holds that resource while it runs, plus the labels the plugin names) and inserts them into the simulation's world, whose `SimStep` runs them in the same order after its clock. The frame's `FixedMain` still runs its order; the moved labels are gone and skipped. A schedule that already ran on the frame's world (its systems bound to that world) is not moved and is reported. Bevy's `signal_message_update_system` moves with `FixedPostUpdate`; the frame sets its message registry's flag after each step instead.

### One id space
The fixed step's data holds entity ids everywhere (parts, holders, targets, the hierarchy) and its systems sort by entity. Mapping each field would mean an entity-mapping implementation on every such type. Instead the two worlds share ids: the simulation's world gets a pool of never-used ids reserved from the frame's allocator (fed into its allocator's free list before it spawns anything, so its own resources take them too; a new world's first two entities, the query filters and the schedules, are the same two resources in both worlds), and the pool is topped up when it runs low. A mirrored entity is spawned at its id on the other side, stepping the index's generation up if the other side is behind; a despawn is mirrored without freeing the id, so each id is in one free list only. `Children` is not copied: `ChildOf`'s hooks keep it.

### Change detection
Each half keeps the tick of its world after its last apply. A gather sends values whose change tick is newer (everything on the frame's first gather), removals read from the world's removal messages (the frame drains them every frame in `Last`, since they are kept two updates), and despawns (removed entities no longer spawned). An apply assigns changed values in place (the receiving systems see `Changed`), inserts new ones (`Added`, hooks), removes removed ones (`RemovedComponents`), then reads past the removals and messages it caused and moves its tick on, so nothing echoes. Lent values are swapped in without a change mark and marked changed only when the giver changed them. Messages are written on the other side; the simulation's message buffers are updated once for every frame update of the frame's.

### Coverage
After moving, the simulation builds the moved schedules and reads every system's component access (data and filters). A type with no registration is listed. Whole-world systems are listed apart: their data (for instance the character tick driver's query) must be registered by hand.

## Risks / Trade-offs

- A type touched by a moved system but not registered is invisible to it (a missing resource fails the system). The coverage list makes this explicit; the game keeps the bridge opt-in until it is empty.
- Query iteration order in the simulation's world can differ from the frame's (different archetypes), so systems that depend on iteration order without sorting may diverge; the tests compare results across worlds.
- Cost: lent types move every step (the swap of a body's fields, not its parts); copied types only when changed; the first step copies everything once.
- Observers and component hooks registered on the frame's world by plugins are not in the simulation's world.

## Open questions

- None for reverse engineering: the change follows the original's division (simulation on its own thread) without adding a rule. Which thread the original's GUI actions run on stays open for stage 6 (REA: callers of the deferred-call ring's post).
