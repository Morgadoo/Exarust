## Context

Follows `bridge-the-simulation-world`. No `docs/formats/` page is involved.

## Decisions

- Registration lives with the plugin whose systems touch the type; registering twice is harmless (types are kept once). The groups were derived from `--sim-boundary`'s "Bridge coverage", which now names each unbridged type's systems.
- Kind per type: messages forwarded; resources lent when they have a default (a swap with a spare, no clone), else copied; components copied when cloneable (only changed ones cross), lent otherwise. Trace files, which only the step writes, move to the simulation once.
- Bodies were lent first: every body of a level crossed twice a step (4936 in exanima01, 12 ms). Copied, only the awake ones cross (about 40 a step).
- Run conditions are read from the schedule graph after it initialized them and before the build moves them out.

## Risks / Trade-offs

- The bridged duel follows its own trajectory (run to run it repeats like main does). Ruled out by experiment: missed changes (`EXA_BRIDGE_ALL=1` gives the same trajectory), `Children` order, run conditions, unregistered types. Candidates left: query iteration order in systems other than the solver, sector lists and character tick; required components the simulation inserts with defaults (WoundQueue, CombatScore, WearsUnderwear, visibility) on its own entities.
- Copy cost after the bodies: whole-world change scans of transforms, names and the hierarchy (~1-2 ms a step in exanima01).

## Open questions

- None for reverse engineering.
