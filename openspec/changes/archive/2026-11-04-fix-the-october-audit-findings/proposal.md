# Proposal

## Why

A rating audit of the commits made directly to main on 31 October and
1 November found probable bugs: the death fade could pause the game before
the player's death and nothing reset it, so a pit death froze the game; a
shock-weapon blade's hit skipped the shock routine; the fist cursor nudge
tested the fists' bit instead of a class word of zero; the knockback step
ignored encumbrance and the leg extension; the kits wrote dirt and wear
into a private database copy; and three comments were stale.

Phases 4 (items), 6 (combat) and 7 (thaumaturgy), and the session flow.
No docs/PLAN.md edit (the main session owns it).

Evidence (REA, Ghidra on `Exanima.exe`, and the assembly listing): the
session's player-death hook and the HUD's fade routine (its states, the
pause counter it raises once), the fade's reset and its callers (the
session leave, the story's end-menu exit, the story respawn's scene reset,
the in-game page's exit, the arena's leave), the frame clock and the
simulation's tick scheduling on the same millisecond timeline, the item
hit hook (the shock routine at 0.01f along the striker's tracker slot
vector), the swing routine's nudge branch (the class word compared with
zero, per striker with the window bit), the moveset routine's class words,
and the feet routine's scaled step length for a step of kind 2
(constants 1.5 and 0.3, read as doubles).

## What Changes

- The death fade runs on the body ticks' game clock, so it completes (and
  pauses the game, once) only after the player's death tick; the player's
  body leaves the scene but stays the session's dead player, so the story
  hook opens the end menu; the fade's reset (`Step::ResetDeathFade`) runs
  in the session leave, first in the story's end-menu exit and first in
  the story respawn, and gives the game time back. The end-of-session menu
  draws over the fade.
- A shock-weapon blade's hit sends the full shock (`ArcShocked`) at 0.01
  along the striker's tracked swing velocity.
- The fist nudge pushes the ticked character's open strikers only while its
  weapon class word is 0.
- A knockback step's length is (1.5 − 0.3·encumbrance) times the step
  length scaled by the leg extension.
- Kit dirt and wear writes reach `DollItems` (tagged ids keep their own
  copies and container lists) and the card's wear byte.
- Stale comments: `PRACTICE_SPOT` (centimetres, wired by the practice
  partner's plan, which now uses it) and the kind 5 take use (the
  divination reading; `TakeUse::Divination`).

## Out of scope

- Saving kit writes and per-copy container lists: they need the live
  object database (S2); copies of an untagged template still share one
  record, and a kit's local copy id may collide with the level's own
  local records.
- The two-hander grip's head-height mark: not traced (not cheap).
- The worn model does not draw dirt or wear in Exarust.
