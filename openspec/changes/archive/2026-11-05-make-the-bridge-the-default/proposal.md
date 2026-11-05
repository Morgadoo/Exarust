## Why

Stage 3, increment 3 of the simulation thread (`register-the-fixed-step-with-the-bridge`). With every type registered, the bridged arena duel still drifted from the one-world duel (about 0.9 degrees from its first traced tick). The drift had to be found and removed before the fixed step could run in the simulation's world by default.

## What Changes

- `--sim-digest <file>` (`exa_physics::sim_digest`): after every fixed step, a hash of each bridged type's values (entity ids left out, values sorted), in a schedule after `FixedLast` that moves with the fixed step; `EXA_DIGEST_DUMP` writes the values. `--hold-loading` holds game time while the loading screen shows. A one-world run and a bridged one compare tick by tick.
- The digest showed the player's steering never ran behind the bridge: query filters (`With`, `Without`) were not in the coverage, so unbridged markers (`Player` and twelve others) matched nothing in the simulation's world. The coverage and the boundary map now read query filters; the markers are registered.
- The unbridged-component warning names the components beside it. The visibility components the simulation's entities carry are required-component defaults on mirrored character roots, not read or sent back.
- Changed values are found through the storage's change ticks (`Changed<T>` with each half's own tick): the transforms', names' and hierarchy's scans cost a third. Cloths are lent instead of cloned.
- The game runs its fixed step behind the bridge by default; `--no-sim-bridge` keeps it in the frame's world.
- Types left out of the digest (no `Debug`, or large read-only data) register as opaque.

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the fixed tick's cost.

Evidence: no engine rule is added. Parity: the duel's joint traces behind the bridge against this build's one world and against main; the digests; per-level skip logs.

## Out of scope

- Commands instead of frame writers and snapshots instead of copies out (stages 4-6); running the simulation free of the frame.
