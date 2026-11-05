## Why

Stage 4 of the simulation thread, coupled writers (`rise-in-the-simulation`). The hit chain (wounds, blade tracking, hit detection, the reaction and the bites) writes bodies, fighters and conditions once a frame in `PostUpdate`, and the brains' reaction to blows and the debug pauses read its results in the same frame and write simulation data in turn. The original takes a pending hit in its dynamics' post-step, on its simulation thread, so the chain and those readers move there.

## What Changes

- The hit chain runs in `SimFrameStart` (in `CombatSystems`), followed by `react_to_blows` and the debug pauses (`arm_pause_on_hit`, `arm_pause_after_collapse`, `refill_undying`).
- `SimFrameStartSystems` (`Early`, `Late`, `Bodies`) keeps the moved work in its old frame order: the rise, then the hit chain and its readers, then the bodies' frame start that queues the blows they sent.
- `FrameStartStep`: `SimFrameStart` runs from a step of the fixed order (after the lockstep, before the commands) on the first fixed pass of each frame, in both worlds, instead of from the frame's before-loop systems, where its order against the other before-loop systems varied from run to run. Bevy's frame count is bridged.
- The chain's own messages and data (blood, wounds, combat score, bites, pains, clashes, layers, shield rings, hooks, taught reactions) are bridged.
- The unbridged-component warning skips one-shot systems' entities and Bevy's hierarchy bookkeeping.
- Presentation stays in the frame: blood painting and spray drawing read the chain's results a frame later.

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the simulation thread.

Evidence: the original's damage routine (`TMotileDynamics`) consumes the pending hit in its post-step; hit events come from the pair stage's contacts; both run in the simulation's step (`context/re/dyn-damage.md`, `body-pairs.md`). No rule changes: Exarust keeps its once-a-frame cadence for the chain.

## Out of scope

- Running the hit chain per tick as the original does (a rule change).
- The hand, drops, hotkeys, use, flow and the golem alarm; snapshots; narrower scans.
