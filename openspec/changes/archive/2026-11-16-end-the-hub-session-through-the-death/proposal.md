## Why

Lane `bridgeaudit`. A `--hub-enter 0 --arena-auto` career run on the build
worker lost its first match: the manager collapsed and died with his health
above 0.01, a fall that is not fatal, so the end screen's Continue returned
to the hub as the original's does (only a fatal fall leaves for the title
from the arena, round 260). The manager's body crosses the level change as
the original's manager object does, still dead, and the hub's session hook
found him dead: "the manager fell; the session ends". That much is the
original's rule. But Exarust's hook only set the session's phase: no tip,
no blocked controls, no end menu, so the hub played on, and the auto
rematch (an Exarust-exposed dormant switch) then launched a match with an
empty lineup over the ended session. The original's hook calls the player's
death routine, which sweeps the screens, shows the fall's tip, blocks the
controls and opens the end menu, the way out to the title. The hook also
ran during the hub's load, which the original's game frame does not.

## What changes

- The hub's session hook ends the session through the player's death
  (`Flow::die`: the screens swept, the tip, the controls blocked, the end
  menu), after its wipe.
- It does not run while a level loads.
- The auto rematch starts nothing once the hub's session has ended
  [inference: the dormant switch's timer is not traced past the end].

## Evidence

The session hook and the death routine read from `Exanima.exe`
(`context/re/arena-hub.md`); worker GPU career runs before and after
(`context/artifacts/2026-11-16-bridgeaudit/`).
