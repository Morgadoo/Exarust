## Context

Follows `register-the-fixed-step-with-the-bridge`. No `docs/formats/` page is involved.

## Decisions

- A headless one-world vs bridge app with the full plugin set is not available: the game's plugins need the renderer and GPU runs go through the grid. Both modes are deterministic under `--fixed-frames` (one world matches main exactly), so the harness is a per-step digest written by the game itself in both modes, compared offline.
- Entity ids differ from run to run (a load spawns in a varying order): the digest strips them from each value's `Debug` text and sorts the values.
- Query filters come from each system's filtered accesses (`with_filters`, `without_filters`); Bevy's default query filters (`Disabled`, `IsResource`) are covered.
- The sorts by entity added in increment 2 stay: they change no one-world result and keep the order independent of table layout.

## Risks / Trade-offs

- Copy cost: about 2.3 ms a step in exanima01, 3 ms in exanima06, 1 ms in the arena, on the simulation's step until stage 6 takes the step off the frame.
- A new fixed-step system touching a type no plugin registers is reported at start and skipped with one warning; the bridge's tests and the start log catch it.

## Open questions

- None for reverse engineering.
