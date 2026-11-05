## 1. Bridge

- [x] 1.1 `SimBridgePlugin`: move the fixed step's schedules (and named ones) whole into the simulation's world; run them in order in `SimStep`; report schedules that already ran
- [x] 1.2 Copy, lend, relationship, resource, message and sim-only registrations (`SimBridgeAppExt`)
- [x] 1.3 Gather and apply with change ticks, removals, despawns, messages; no echo of a side's own applies
- [x] 1.4 One id space: reserved id pool, mirrored spawns and unfreed despawns, generation catch-up
- [x] 1.5 Coverage of the moved systems' data; `BridgeCost`; `--sim-bridge`; `--sim-boundary`'s bridge coverage

## 2. Groups behind the bridge

- [x] 2.1 Bodies (`BodyPlugin`): bodies, collision world, solver stages, shove queues, sector lists, messages
- [x] 2.2 Character tick and collapse (`CharacterTickPlugin`, `CollapsePlugin`), Bevy's hierarchy, transforms, names and clocks
- [ ] 2.3 The rest of the fixed step (the coverage list in the evidence), then `--sim-bridge` by default

## 3. Parity

- [x] 3.1 Bridge semantics: a scripted run logs the same changes, additions, removals, spawns, despawns and messages on both sides as one world, inline and threaded; ids shared; unchanged values not copied
- [x] 3.2 Body test: every pose bit for bit with the fixed step in the simulation's world, inline and threaded
- [x] 3.3 Stage test: the character tick's posture-branch death through `FixedMain`, alike in one world and behind the bridge, inline and threaded; no type missing
- [x] 3.4 Grid: the default path (bridge off) against main; `--sim-bridge` started in the grid with its coverage logged

## 4. Docs and checks

- [x] 4.1 exa_physics and exarust READMEs, docs/DEBUGGING.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 4.2 Corpus check: not applicable (no data format is read); the grid runs read the real installation
- [x] 4.3 fmt, clippy -D warnings, tests of the touched crates on the worker (`make check`'s parts)
