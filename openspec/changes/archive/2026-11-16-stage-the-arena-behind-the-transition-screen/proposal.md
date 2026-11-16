## Why

Roadmap task B4 (lane `arenafade`, docs/roadmap/phase-b-close.md), after the
`aifreeze` lane (round 251): every practice and arena staging runs as the
transition screen's callback, but Exarust's arena rounds and match start
opened no transition screen (no 600 ms fade-out before the staging, no
overlay, no paused frames), and the staging freeze was counted as 92 ticks
instead of ending with the screen.

## Evidence

The disassembly (`context/re/staging-freeze.md`, `context/re/exe.asm`):

- The arena end menu's Continue raises the round counter and passes the
  next round or (tournament flag set) the bracket round to the transition
  screen's opener with the music fade and the short hold off. Its return to
  the hub and its dead-manager exit pass their own callbacks (music fade
  on).
- The rematch and the hub's Begin pass the match start with the music fade
  on and the short hold off; the match start loads the arena inside the
  callback.
- The next round, the bracket round and the match start raise the pause
  counter only while it is zero; the screen lowers it (while above zero) on
  its second frame after the callback, then holds 500 ms, fades back in
  over 600 ms and clears the staging byte when the fade ends.

## What changes

- `exa_ui::transition`: the step reports the frame that lowers the pause.
- `exa_game::flow`: `Action::ArenaRound` and `Action::ArenaMatchStart`, the
  staging step `Step::ArenaStage`; a staging callback raises
  `SimPause` (while zero), the screen's frame lowers it, and the close posts
  the freeze's release. The match start holds the screen black over its
  level change. `Step::FreezeAi` under a running screen holds the freeze.
- `exa_game::arena_fade` (new): holds the end menu's next round and the
  hub's launch until the callback; the host's level change reads the new
  `ArenaMatchStart` message.
- `exa_game::arena_live`: the Continue opens the screen, closes the end
  menu and stages the round at black; the first round and the next round
  hold the freeze under a screen.
- `exa_ai::freeze`: a held freeze, ended only by the release; the 92-tick
  count stays for a staging with no screen (a command-line match, the
  practice start).
- `exa_game::sim_control`: the lockstep fixed loop does not run while the
  pause counter is raised.

## Determinism

The transition screen runs on the frame's real clock. In scripted runs
(`--fixed-frames`) that clock advances 1/60 s a frame, so the release
comes at the same frame, and so the same tick, every run, and the release
is posted through the fixed queue like the staging. Lockstep stays
deterministic; the screen-timed end is kept.

## Impact

Arena traces change: each next round and match start runs 600 ms later
(after the fade-out), three frames are paused after the staging, and the
freeze ends with the screen's close (about 1100 ms after the pause lowers)
instead of after 92 ticks. The practice next fight's traces change by its
three paused frames and the screen-timed release. Determinism runs owed:
the arena scenes (`--arena-auto`), the arena hub's Begin, the practice next
fight.
