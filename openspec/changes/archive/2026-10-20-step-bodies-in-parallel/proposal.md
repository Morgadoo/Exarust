# Proposal

## Why

With character shadows cached, play mode in exanima01 was bound by the main
thread, and the body solver took 7.5 ms of every 83 Hz tick. A trace split
it: 97 % of the 20 link iterations is each body's own links, and level
detection, integration, friction and the post-step are per body too; only
the pair pass and the pair friction couple bodies. The original simulates
every active character every tick (levels load whole, so every sector
counts as loaded), so the tick's work cannot be skipped, only shared out.

## What Changes

- Each solver stage says which bodies its pair passes, links and friction
  touch this tick (`SolverStage::coupled`; every body unless a stage says
  otherwise). The pairs mark the bodies of their pairs.
- A body no stage touches runs its 20 link iterations at once, and its 8
  level friction passes at once, in parallel with the other free bodies on
  the compute pool. Coupled bodies keep the original's interleaving with
  the pair passes.
- Integration, level detection and the post-step run in parallel across
  bodies.
- Tracing spans name the solver's phases for the frame profiler's tracing
  builds.

The arithmetic of every body is unchanged, so every result is the same
bit for bit.

## Capabilities

### Modified Capabilities

- `physical-body`: the world tick's per-body work runs in parallel.

## Impact

- `exa_physics::body` (`solve_among`, `SolverStage::coupled`, the `Sync`
  bound on the level), `exa_physics::body_pairs` (`coupled`, spans).
- `docs/PERFORMANCE.md`.
- Evidence: `context/re/optimizations.md` §2 (levels load whole);
  `context/re/component-map/sections/ai.md` (activation within 200 m and
  in a loaded sector; living bodies never sleep, `context/re/body-misc.md`
  §3).
- Out of scope: the 200 m activation itself (no effect in a loaded
  dungeon), parallel pair contacts.

## Phase

Phase 8, performance.
