## Context

The design and the stages are in `plan-the-simulation-thread` and the lane's artifact (`2026-10-30-simthread/1-DESIGN-PART.md`, `2-SHELL-PART.md`). This is stage 2.

## Decisions

- The shell owns a `SubApp` rather than adding one to the `App`: `App::update` would otherwise run and extract it once a frame, while the simulation steps once a tick.
- Lockstep through a schedule of its own inserted in `FixedMainScheduleOrder` before `FixedFirst`, so no existing schedule gains a system or a sync point, and each pass of the fixed step (including a paused step key's single pass) is exactly one simulation step.
- Commands and events are boxed `FnOnce(&mut World) + Send` closures: one queue each, ordered, with no per-type plumbing. Typed commands can be added where a stage needs them.
- The thread mode is the default: it exercises the hand-offs across threads, and in lockstep it waits for each step, so results cannot differ from inline.
- A latest-wins slot for snapshots (the simulation never waits for the frame) and an ordered queue for events (none lost or doubled when several steps run between frames).

## Open questions

- None new; stage 3's ownership of `Ragdoll` (12 fixed-step and 22 frame writers) is planned in the artifact.

There is no `docs/formats/` page for this change: it is runtime structure, not a data format.
