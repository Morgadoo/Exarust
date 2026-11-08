## Why

Every lane's evidence compares `--fixed-frames --trace-all` traces against main, and main was bimodal on exanima01:
four identical scenes in one grid split two and two, 4,230 rows differing from tick 121 on. The values were never
different: the rows of tick 121 onwards came in another order. At tick 121 the spawn watch (`limp`) removes its
marker from five character bodies by command, in its query's order; in one run the player's body led the five, in
the other it followed them, so the simulation's archetype for the five (and the trace's rows) took two orders. The
simulation's archetypes were built in the order the bridge applied each type's values, and the frame gathers those
in its own query order, which hangs on the frame's load-time archetypes (meshes, lights and the GUI arrive in
whatever order loading finishes). docs/PLAN.md Phase 3, the simulation thread.

Evidence: `EXA_ARCHETYPE_TRACE=1` (new) on four identical scenes: the only order difference is the `SpawnWatch`
removal at tick 121; the two modes' traces hold the same rows per tick (sorted, zero differences).

## What Changes

- The simulation applies each bridged component's values and removals (copied and lent) in entity index order, not in
  the order the frame's queries found them. The frame side keeps its order (its archetypes do not feed back).
- `EXA_ARCHETYPE_TRACE=1`: logs each character body whose components changed in the simulation's world, each tick,
  in the order the bodies moved.
- `context/tools/determinism_grid.sh`: runs 2–4 identical lockstep scenes in one grid and compares their traces.

Rule change: none: every tick holds the same rows (values and body ids) as both of main's modes. The arena duel's
trace is byte-identical to main's. exanima01's rows now come in index order within each archetype (main's frame
order was the reverse), so its trace equals neither old mode byte for byte; sorting by `Entity`'s own order instead
(index descending) reproduced main's first exanima01 mode but swapped the duel's two rows for its first 120 ticks,
and the duel is the reference every lane compares.

## Out of scope

- Message order: messages the frame writes in its query order still cross in that order (none seen to differ).
- Free mode, which is not tick-identical by design.
