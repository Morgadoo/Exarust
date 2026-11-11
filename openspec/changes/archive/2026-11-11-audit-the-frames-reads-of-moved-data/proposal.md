## Why

The aitrig lane (round 160) found that the story revive's brain reset ran in the frame's world and read the
navigation grid through an optional resource: behind the simulation bridge the grid is moved to the
simulation's world, so the reset always read nothing and never moved a brain. Any frame system that reads a
type the bridge moves may fail the same way, silently. Phase 0 (the simulation thread) in docs/PLAN.md.

## What the original does

Nothing to port: the original runs every scene's simulation on its own thread and its other threads hand it
work through the ring of deferred calls. Exarust's bridge keeps the same split; a frame reader of the
simulation's own data is a port defect, and the fix is the original's: post the work to the next step.

## What changes

- The bridge's types report whether the frame's world holds them (`BridgeHalf::frame_absent`): a moved
  resource and a simulation-only type never, a lent resource not while the simulation runs free.
- `sim_bridge::audit::audit_frame_reads`, in the frame's `First` and `Last`, checks each frame schedule as it
  is built (and when its systems change), skipping the start-up schedules, for systems whose access names
  such a type; it logs the types and each reader (an error), and stops a debug build unless the reader was
  allowed (`FrameReadAuditAppExt::allow_frame_read`). Its state lives in `FrameBridge` (a resource of its own
  would take an entity id and move the ids of lockstep runs).
- Two readers found, both silently broken behind the free-running simulation (the default):
  - `--weapon-blood` (`blood::bloody_debug_weapons`) read the weapons' blood list (moved): the weapon took
    the blood in no spot. It now posts `stain_debug_weapon` to the next fixed step.
  - The profiler's probes read and cleared the brains' clock (`BrainTickClock`, lent: it stays in a free
    simulation), so the `brains` line read 0. The clock is now a snapshot whose totals only grow, and the
    profiler reports the difference from the last frame's.
