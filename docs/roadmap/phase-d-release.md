# Phase D — Performance, platform, release

**Goal:** a build someone else can install and play.

This is Phase 8 of [`../PLAN.md`](../PLAN.md), ordered. Two items (D1.1,
D1.2) are needed early, by Phases B and C.

## Tasks

### D1. Working infrastructure (early)
1. **Binary retention**: keep the last three round binaries; a lane names
   the one it compares against, and cleanup skips it.
2. **GPU run queue**: one queue for grid runs with a memory check, so
   three lanes cannot run the card out of memory. Several checks on
   2026-11-11 failed for that reason alone.
3. **Test count**: the suite reports 8,500 or 9,000 tests depending on
   whether the worker's game-data tests run. Make the report say which.
4. **Old lane folders** on the build worker and three stale worktrees:
   confirm ownership, then remove.

### D2. Performance
1. A fixed set of heavy scenes (a full story level, an outdoor arena, a
   crowded fight) with a frame-time budget each, measured alone on the
   card, not in a shared grid.
2. Track frame time per round for those scenes; a round that worsens one
   by more than 5% needs a reason.
3. Work the order in [`../PERFORMANCE.md`](../PERFORMANCE.md).
4. Loading times per level, against retail.

**Done when:** each heavy scene meets its budget at the retail settings.

### D3. Platform
1. Settings: every option of the original's settings screen has its
   effect.
2. Gamepad: the action sets are reproduced on gilrs; verify each binding
   with a real pad. Steam Input follows decision DEC-2.
3. Display: fullscreen modes, the custom cursor, the error message box
   (listed missing in the component map's runtime section).
4. Windows and macOS builds, after Linux is complete.

### D4. Regression suite
1. Recorded inputs for the playthroughs (C5), replayed in lockstep each
   round, compared by trace.
2. The feature sweep and the scenario matrix as one command with one
   report.
3. The picture set (C2) re-shot each round on the Exarust side; changes
   above the noise floor are listed.

**Done when:** one command says whether a round changed behaviour.

### D5. Packaging
1. A release build that finds a Steam installation by itself.
2. No game data in the package; the user's own copy is required.
3. Loose-file overrides for modding (already planned in Phase 8).
4. Crash reports that carry the log and the scene.

### D6. Final review
1. The ledger has no open row.
2. The scenario matrix has no failing row.
3. Both playthroughs pass, played by the owner.
4. The documentation matches the code: specs, crate notes, the component
   map.

**Done when:** the owner signs off the four points.
