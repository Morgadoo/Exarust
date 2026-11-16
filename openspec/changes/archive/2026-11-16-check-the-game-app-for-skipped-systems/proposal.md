## Why

Lane `bridgeaudit` (findings F547, F548). From round 253 to round 276 the
casters' walk, the vanish turns and the bodies' impact sounds never ran in
the game: the simulation world lacked a message queue and a resource they
need, and Bevy skipped them every tick with one run-time warning. Every
workspace test passed, because the tests build their own small Apps, and
the bridge's coverage check only stops a debug build of the game, which no
test built.

## What changes

- `exa_physics::skip_watch`: handlers that record every system (or run
  condition) Bevy did not run for a parameter that failed validation, with
  the world, the system and the parameter. A frame world holding
  `WatchSkips` makes the bridge install the simulation's handler at its start
  and record its uncovered types instead of stopping.
- `exarust`: the command line's reading (`parse_command_line`) and the App's
  building (`build_app`) leave `main`, so a check builds the game's App from
  a command line exactly as `main` does, headless (no window, no GPU) when
  asked.
- `exarust::app_skips_test`: builds the game's App headless with the bridge
  in lockstep, runs the arena without an installation (everywhere) and, with
  `EXANIMA_DIR`, the arena with a duellist and a Shock cast and the first
  level, 300 ticks past each load, and fails on any system not run, listing
  each with its missing parameter. Only systems that need the installation
  may be skipped in the data-free scene.

## Evidence

On the round's code the check passes: no skipped system in either world in
the three scenes (the data-free arena skips only the five systems that read
`Game`). With the vanish turn's bridge removed again it lists the casters'
walk and the vanish turns. Its first run also met a debug overflow in the
GI shape placement while the Shock's burst lit the arena (F552).
