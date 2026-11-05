## Why

Stage 3, increment 2 of the simulation thread (`bridge-the-simulation-world`). The bridge carried the bodies and the character tick; 211 of the 261 types the moved fixed step touches were not registered, so `--sim-bridge` skipped 91 systems. This change registers the rest, plugin by plugin, so the whole fixed step runs in the simulation's world with no system skipped, and measures what the copies cost.

## What Changes

- Every plugin with fixed-step systems registers the data they touch (a `bridge_*_types` function next to its build): thaumaturgy, teleporters, NPC search, player, roles' scripts, effects, minds, destinations, limp, falls, simulation controls, traces, body overlay gizmos, brains, combat, characters, audio, mechanisms, navigation, cloth, light sources, the rest of the bodies, the profiler.
- New bridge kinds: a resource moved once to the simulation (trace files), the order of `Children` after `ChildOf`; lent resources swap with a kept spare instead of building a default each step.
- Bodies are copied when they change instead of lent every step (exanima01: 19 ms a step of copying down to 3.6 ms); the navigation grid is lent.
- The coverage check reads run conditions too, and warns once for components the simulation's entities carry unbridged. Each side logs its costliest types every 1000 steps; `EXA_BRIDGE_ALL=1` copies every value every step.
- The solver, the sector lists and the character tick walk bodies in entity order (spawn order), so neither world's table layout decides it; one-world traces are unchanged by it.
- The shell joins its thread when dropped.

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the fixed tick's cost.

Evidence: no engine rule is added. Parity against Exarust's one-world fixed step: the bridge's unit, body and stage tests; grid joint traces of the arena duel; per-level skip logs.

## Out of scope

- The remaining trajectory difference of the bridged duel (reproducible, from before the first traced tick; not missed changes, child order or run conditions): next increment.
- Faster change scans (the transforms', names' and hierarchy's whole-world scans dominate after bodies).
- Commands instead of frame writers, snapshots instead of copies out (stages 4-6).
