## Why

Roadmap task B4 with B1 (lane `gamemain`, docs/PLAN.md Phase B close):
the game main unit's open ledger rows. The practice start's role, the
practice fights' and the respawn's frame heights, the dialogue's end at
the session leave, the first wake-up's flag around the load's GUI reset
and the camera shots' pointer warp had ports the running game did not
apply. Reading the assembly also showed that the flow hid the in-game
GUI where the original sweeps the game screens, so the HUD stayed hidden
after a door, a death or a second session.

## Evidence

REA (Exanima.exe 0.9.5), read in the instructions where constants, order
or branches matter:

- The practice start and the next fight pass the practice scene filter to
  the named level load and to the arena reload; neither callee reads that
  argument register before overwriting it, so no node is ever filtered.
- The practice start clears the player record's role count and adds role
  16 (Unaligned) before the spawn.
- The practice start writes its frame's height as the pose's pelvis
  target plus 5; the next fight writes it twice and keeps the stand-height
  word read after the fight's motion starts; the respawn copies the
  portal's frame (or the identity frame) and adds 15 to its height. Each
  then calls the living placement.
- The session leave and the story's end-menu exit call the dialogue's end
  of every conversation; the leave removes the in-game GUI's frame hook
  near its end. The practice start, the story start, the story continue,
  the hub continue and the new career add that hook after their loads.
- The death, the knock-out, the level exit, the story exit and the leave
  call the game GUI's sweep with argument 1; the application's quit calls
  the container-family sweep. The game layer's leave shows the game's
  view form (argument 1 of the form's visibility call).
- The load's GUI reset clears the first wake-up's flag byte; the story
  start sets that byte after its load.
- The point shots warp the pointer only while the window's active flag is
  set.

## What changes

- `exa_game::flow`: `Step::PlacedFrame` (the practice start's
  `FrameHeight::Target(5)`, the next fight's `FrameHeight::Stand`, over
  the origin), `Step::EndDialogues`, `Step::FirstWake`,
  `Step::SweepScreens`; `Step::PracticeFilter`, `node_stays`,
  `child_stays` and `practice_height` removed; `PracticePlayer`, the
  camera shot's warp and the new steps applied; each start and continue
  turns the in-game GUI on after its load, only the leave turns it off.
- `exa_game::level_change::PlacedFrame`; the viewer's placement skips the
  floor probe for a frame height, and the portal placement uses
  `FrameHeight::Given`.
- `exa_game::load_reset` clears the first wake-up's flag.
- `exa_script::EndAllConversations`, read by the dialogue update.

## Out of scope

- The menu background scene's load (needs the viewer to load a level
  without a player under the menu camera; owed with a GPU check).
- The death screen's player rebuild (Exarust's respawn keeps the body).
- The two-point camera shot and the hide of the main-menu form (their
  callers are the arena unit's).
- The AI staging byte the practice and arena staging set (finding F544,
  another lane).
