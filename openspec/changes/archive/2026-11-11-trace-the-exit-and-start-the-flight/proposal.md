## Why

Round 181, the exitstall lane. With Constant Displacement active, about one
grid in two had a quadrant that logged "frames played, exiting" and stayed
alive 80-125 s while its simulation thread kept stepping; the previous lane's
debugger watcher never caught it. Docs/PLAN.md, Phase 6 (AI), TMotileAI: the
original's flee start (154 bytes) was ported but nothing called it.

## Evidence

- Exit: `EXA_EXIT_TRACE=1` stage times and in-process stack samples
  (context/artifacts/2026-10-30-exitstall): the main thread joins cpal's ALSA
  worker in `Stream::drop` (Bevy's audio output, dropped with the frame's
  world); the worker sits in rodio's resampler for the whole stall. rodio
  0.22.2 computes `from * position` in u32; for the ratio 100157:44100
  (Constant Displacement's phasing sound at pitch 1.068 against the 44.1 kHz
  output) the product wraps and the resampler walks 2^32 frames.
- Flee start, REA (the assembly of `Exanima.exe` 0.9.5): its only caller is
  the target pass. On the pass's flee outcome (its pick's level 4, kept in a
  local; the pass writes no level itself) the pass, after raising the alarm to
  twice the alarm mean, ends the body's conversations, attends the contact
  and calls the flee start, then keeps the flight's alarm at least the pass's.
  The start writes the body's target (a character) or the AI's object focus;
  only when the level is not 4 it halves the courage, adds 2 to the alarm word
  the pass raised and sets level 4; it then marks the target fresh, stores the
  contact's room and installs the flee state.

## What Changes

- `exa_physics::exit_trace` (new, `EXA_EXIT_TRACE`): exit stages with times,
  a watchdog reporting a stage over `EXA_EXIT_TRACE_DUMP` seconds with the
  threads' states and three signal-taken stack samples (then eu-stack/gdb when
  present). Stages marked in `SimShell`'s runner drop and thread, the viewer's
  `--frames` exit, `main`, and marker resources of the frame and render worlds.
- `patches/rodio` (`[patch.crates-io]`): the resampler's two products in u64,
  with a one-time stderr note when a ratio that wrapped upstream is resampled.
- `exa_ai::brain::Npc::run_pass`: the flee outcome runs `flee::start` with the
  level before the pass and the raised alarm, writes the halved courage and
  the alarm back, marks the target fresh and keeps the threat's room
  (`Npc::flee_threat_room`); a fresh flight is logged (`the target pass
  starts a flight`). A debug line (`exa_ai::pass`) gives each pass's outcome,
  morale and highest contact concern.

## Impact

- Exit is prompt (48 of 48 quadrants under 1 s against 6 of 40 stalled).
- Lockstep traces (exanima01, arena duel) identical to main.
- No grid scene reaches a flight on main or here: the brains' contact concern
  stays 0 and the contact alarm is never written, so the flee start runs only
  in tests until those words are live.
