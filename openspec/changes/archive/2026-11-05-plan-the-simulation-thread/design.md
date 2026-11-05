## Context

The full design, with the options weighed and the stages, is in the context artifact `2026-10-30-simthread/1-DESIGN-PART.md`. In short: a simulation sub-app with its own world on its own thread (Bevy's `SubApp`, as the render app), entities named across worlds by a stable id, a frame-to-simulation inbox drained at the top of each pass (the original's deferred-call ring, whose rules `exa_game::frame::DeferredCalls` already ports), a latest-wins snapshot buffer and an ordered event queue back to the frame, and a lockstep mode for `--fixed-frames` and tests so runs repeat tick for tick.

## Decisions

- Two worlds, not one: `Schedule::run` needs `&mut World`, so a second thread over the same world gives no overlap, and running the fixed step beside frame systems through an unsafe world cell is unsound with deferred commands. Rendering is already on its own thread (pipelined rendering); what the tick blocks is the main world's frame work.
- The simulation keeps the authoritative world, not the presentation: the render extraction, transform propagation, visibility and the UI expect their data in the main world, so mirroring them would be larger than moving the fixed step's data.
- The driver is ported as a pure clock (`poll` / `stepped`) so the thread, the frame loop and tests can all drive it.
- The boundary map reads the access Bevy records for each system, so it follows the code as it changes and sizes each stage.

## Open questions

- Which thread runs the original's GUI actions (direct writes or the deferred-call ring): REA, the callers of the ring's post function, before stage 6.
- The frame-side combat systems write fighters' state the fixed step also writes; whether the original runs them per tick: REA, before stage 4.

There is no `docs/formats/` page for this change: it is runtime structure, not a data format.
