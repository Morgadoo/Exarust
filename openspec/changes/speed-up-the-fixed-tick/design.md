## Measurements

Profiles were taken with `perf record` (user cycles, LBR or DWARF call stacks) attached to a grid child running the `profiling` build, and with Bevy's chrome trace (`--features trace-chrome`). The machine was shared with other agents (load averages 5 to 50), so before/after numbers come from one grid running both builds at once (`exe=<old build>` in one quadrant, the new build in another) and the profiles are compared as shares of unchanged work (the cloth stage) rather than as wall time.

## Holder lookups

A held item names its holder by key: the entity of the body's first part. Each system that needs the holder walked every body for every item (`iter().find`), so the cost was items × bodies, every tick. `first_by_key` takes the keys wanted and the query's bodies and returns, for each key, the first body with it in the query's order, ending the walk once every key is found. A `find` per key gives the same body, so duplicates (tests give several bodies the placeholder entity) resolve the same way.

## Pair search

`Part::cell` computes the part's matrix and mesh-box centre exactly as `body_pose` does and returns the same cell. A sleeping part whose cell is far from every awake part is skipped without the rest of its pose; a near one builds its full pose as before.

## What was not changed

- The sector body grid (round 60) rebuilds every part of every body every tick. An incremental rebuild would rely on change detection, which a sleeper the solver pushes without waking for good does not set, so its result could differ for a tick.
- The fixed step's single-threaded executor (`EXA_FIXED_EXECUTOR=single`) did not show a gain under the shared machine's load, so it stays a debug switch.
- A memo of the character sweep (same inputs, same answer) was tried for the sewer; the follow's step and the held torch's stand-in change every tick there, so it never hit and was removed.
