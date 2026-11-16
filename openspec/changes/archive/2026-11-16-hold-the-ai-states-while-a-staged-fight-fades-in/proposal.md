## Why

Roadmap task B4 (lane `aifreeze`, docs/roadmap/phase-b-close.md), finding
F544: the original's AI tick runs an idle stand instead of any state while a
global staging byte is set, and the practice and arena staging set it.
Exarust's arena and practice duellists ran their combat state from the first
tick of a fight.

## Evidence

REA (Exanima.exe, Ghidra) and the disassembly (`context/re/staging-freeze.md`):

- The byte has one reader, the AI tick's test right after the contact
  integration (which runs the target pass: the combat entry and the
  set-target happen before it). Set, the tick calls the idle stand in place
  of the current state's call: a body following a route soft-stops, and a
  present body takes the out-of-combat turn toward its look with the idle
  sway. The walk after the state runs either way.
- Five routines set it: the practice start and the next practice fight, and
  the arena's next round, bracket round (from the second round on) and
  match start. Each is the transition screen's callback, run once the screen
  is black, and each raises the pause counter.
- Only the transition screen's run clears it: after the callback it passes
  two frames, lowers the pause counter on the third, holds 500 ms (no hold
  flags for these callers), fades back in over 600 ms and clears the byte on
  the frame the fade ends. There is no countdown or cue.

So the fighters stand frozen in the idle stand for the hold and the fade,
about 1100 ms of simulation (92 ticks at 12 ms).

## What changes

- `exa_ai::freeze::StagingFreeze`, owned by the simulation and staged from
  the frame through the fixed queue; the first system of the brains' chain
  counts 92 body ticks and clears it.
- While it holds, the AI tick runs the turn (and Exarust's model turn) in
  place of every state; the duellist runs only its combat entry; the flight
  and idle walks do not run; a followed route soft-stops.
- The flow's practice start and next fight end their staging with
  `Step::FreezeAi`; the arena's first-round, next-round and bracket staging
  set it.

## Impact

Every arena and practice trace changes: for the first 92 ticks of each fight
the opponents enter combat (guard, target) but do not approach, strike or
walk; they turn in place. Determinism runs owed: the arena and practice
scenes.
