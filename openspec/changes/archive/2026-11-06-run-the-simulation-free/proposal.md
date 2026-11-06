## Why

Stage 6 of the simulation thread (docs/PLAN.md Phase 3, bodies: the simulation thread). The original runs its
simulation on its own thread at 83 Hz on a real clock, and its frame draws between the last two published steps.
Exarust's simulation world still steps in lockstep, driven and waited for by the frame's fixed loop, so a slow
step costs the frame its rate. Every lent type now has one owner (step 1a); the ones only the simulation touches
can move there for good, and a free loop can run behind a flag.

Evidence: REA of Exanima.exe 0.9.5 (context/re/sim-thread-input.md): the simulation loop (pause counter, 11 ms
sleep while paused, posted-call ring drained before each step, 12 ms waitable step, backlog dropped past four
steps, published tick and time), the frame's render fraction (1 + (clock − published time)/step, clamped to
[−2, 2]) and the bodies' interpolation between their last two history entries; the pause counter's writers
(loading/transition screen, main menu, splash, character creation, end-session menu, arena screens, in-game GUI
mode 2).

## What Changes

- The lent resources only the simulation touches move to the simulation in every mode (`bridge_move_resource`):
  brain clock, footprints, room grid size, activation area, the extra encounter script, the sampled pose, the
  teleport latch, the shove queues, the sector lists, the cloth objects and contacts, the moving meshes (their
  removal posted first, as the grid's). The alarm's zones are copied in and its state is a snapshot (the frame's
  flow reads them). The profiler's tick count is a shared counter. Each world keeps its own `FixedCommands`; the
  frame's posts and writes go in its packet.
- `SimMode::Free` (`--sim-free`; lockstep stays the default): the original's loop on the `exa-sim` thread
  (`step_driver`'s clock, `SimPause` hold, packets drained before each step, `FreeStamp` after it, its own
  `Time<Fixed>`). The frame's fixed loop is off; `exchange_free` sends the frame's packet without waiting and applies
  every arrived step packet in order, once a frame (`SimExchange`).
- Interpolation: the original's render fraction becomes the frame's `Time<Fixed>` overstep (0–2 steps), so every
  drawing reading `overstep_fraction` draws the original's moment: one step behind the clock (the original draws between its two older history entries at the fraction; Exarust's bodies keep two, drawn at fraction − 1).
- Data both sides write: drawn copies (`bridge_copy_drawn`: transforms, the drawn controller) keep a newer
  simulation value; other values take the frame's write as in lockstep; both counted in the log.

Rule change: none in lockstep (every body's every trace column identical to main in the arena duel, exanima01,
exanima02 and exanima06). Free mode is not tick-identical by design.

## Out of scope

- Free mode as the default (stage 6 step 3), until free mode shows no artefacts and lockstep parity holds.
- Negative render fractions (the original's −2..0 range): the overstep cannot be negative, so it is held at 0.
- Single-stepping and slow speeds with the pause key act on the frame's clock only in free mode.
- The creator screen's own pause raise beyond the hold (another session owns that screen).
