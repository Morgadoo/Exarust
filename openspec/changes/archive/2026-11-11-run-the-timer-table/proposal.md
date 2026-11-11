## Why

Phase 8 (performance and settings) in docs/PLAN.md, RF_Main and
FLX_System. The main unit's periodic timer table was modelled in
`exa_game::frame` but nothing ran it: the GUI screens ran the GUI timer's
work (the delayed tooltip, the tooltip's auto-hide, a held scroll arrow's
repeat, the deletion of closed forms) every frame instead of every 100 ms,
and the arena's auto rematch kept its own due time.

## What the original does

REA and `exe.asm` of the main unit's timer routines (register, free,
reschedule), the frame tick's run loop, their callers and the callbacks:

- **The run** happens once a tick, after the frame is paced: each record
  that is not free and whose due time is at or before the frame's
  real-time clock (an unsigned compare) runs its callback with its record
  and is armed `interval × 100` from that clock. Registering takes the
  first free record or appends one and returns its one-based index; a free
  sets the record's flag; a reschedule arms it `(interval − elapsed) × 100`
  from the frame time.
- **Three units register records.** The GUI's start registers one unit
  (100 ms) for good; its callback is the GUI timer: the delayed tooltip
  (400 ms after the request, focus unchanged), the tooltip's auto-hide,
  the pressed and the timer form's timer methods and the deletion of
  closed forms. The item container registers two units for its delayed
  card, frees the old request first and frees the record once it fired.
  The arena registers forty units on the return to the hub when its
  (never set) auto switch is on; the callback frees its own record.
- **A press on a scroll arrow** makes that form the timer form and
  reschedules the GUI timer by −6 units, so it next runs 700 ms later.
- **The shutdown hook chain** dispatched when the window is destroyed (set
  once GL started) is joined by the GUI, the texture, the settings and the
  character-generation units: the settings unit deletes its GL objects,
  the character generator only passes the call on. The render-target and
  viewport stacks, the GL error check, the swap-interval switch and the GL
  start-up are GL plumbing.

## What Changes

- `exa_game::frame::PeriodicTimers<T>` holds an owner tag per record and
  its run returns the due records in record order (no callbacks).
- `exa_game::frame_timers`: `FrameTimers` (the GUI timer registered as
  record 1), run in `First` on the app's real clock; `EXA_TIMER_TRACE`.
- `exa_game::form_input::drive` runs the forms' timers and the task's tick
  only on GUI-timer frames and reschedules the GUI timer when a press
  starts an arrow's repeat; the game screens, the manual and the Settings
  Controls page pass the table.
- The arena's auto rematch registers and frees a 40-unit record.
- The engine-replaced routines (render-target stacks, GL error check,
  swap interval, GL start-up, shutdown hook chain) are documented.

## Capabilities

### Modified Capabilities

- `frame-loop`: the live timer table, the GUI timer and its arrow pause.

## Impact

- Code: `exa_game::{frame, frame_timers, form_input, gui_screens,
  help_live, settings_form, arena_hub_live}`.
- Docs: `docs/PERFORMANCE.md` (Frame pacing), `docs/DEBUGGING.md`, the
  exa_game README.
- No simulation change: the table and its users are frame-side only.
