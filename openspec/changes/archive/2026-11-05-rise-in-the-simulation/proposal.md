## Why

Stage 4 of the simulation thread, coupled writers (`post-frame-writes-as-commands`). The rise writes a corpse's body, condition and fight state, and its restore reacts in the same frame by giving back the clips and the controller; posting only the body's revival would let a revived body tick once without them. The original runs the rise in its AI manager frame on the simulation thread, so the pair moves there.

## What Changes

- `rise_the_dead` and `restore_revived` run in `SimFrameStart` (once a frame, before its ticks; in the simulation's world behind the bridge), chained, instead of in the frame's `Update`.
- `SimFrameStart` runs its systems one at a time in order (its writers share the bodies).
- The corpse's kept records (`LimpKept`) and death clock are bridged.
- `--schedule-ambiguities` also reports the frame's `Update` and `PostUpdate`; `EXA_FRAME_EXECUTOR=single` runs them one system at a time.

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the simulation thread.

Evidence: REA on Exanima.exe: the AI start-up chains the manager frame into a global hook (keeping the previous hook), and the simulation thread's body reads and calls that hook in its step, so the original's rise runs on the simulation thread, each step. Exarust keeps its once-a-frame cadence for now.

## Out of scope

- Running the rise every tick as the original does (a rule change, traced separately).
- The hit chain, the hand, drops, hotkeys, use, flow and the golem alarm (next increments).
