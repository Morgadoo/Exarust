## Why

Stage 6 of the simulation thread starts by matching the original's cadence. REA shows the AI manager's frame routine (the rise and its restore, then the alarm zones) is the original's simulation loop's pre-step hook, called before every 12 ms step; Exarust ran it once a drawn frame.

## What Changes

- A `SimPreStep` schedule runs before every fixed step, in both worlds (moved with the fixed step behind the bridge). The rise, the restore and the golem alarm's zone check run there.
- The hit chain, its readers and the bodies' frame start stay in `SimFrameStart` (first step of a frame) for now: the blade tracking reads the frame's propagated blade frames.

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the simulation thread.

Evidence: REA, `context/re/sim-thread-input.md` (the AI start-up installs the manager frame as the loop's pre-step hook; the routine has no rate guard: every call walks the characters, the zone check gated by its 3 cm movement test). Rule change: the rise and the zone checks run at 83 Hz instead of once a frame.

## Out of scope

- The hit chain per step at the dynamics' post-step (needs blade frames from the parts in the step); the free-running loop.
