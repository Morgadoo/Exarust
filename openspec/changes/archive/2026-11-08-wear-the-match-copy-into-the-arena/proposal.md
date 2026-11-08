## Why

PLAN Phase 7 (arena career), gap lane `arena`. The XA_Arena routine table
(lane report) found the match start's loadout pass uncalled: every company
entrant stood as an actor template in its clothes and arms, whatever the
fighter's loadouts held. The tournament start's rival-winner pass, the board's
forfeit question and the dead-body sweep before Continue and forfeit were
ported but had no caller.

## Evidence

REA (Exanima.exe), read in the assembly: the match start's loadout pass copies
each participant's record with its item and loadout lists emptied, sets the
copy's fighting skill ((levels + 1) × 10 under ten levels, else 100), and puts
on the first loadout with the four hand item words zeroed when the tier is
below 0, else the loadout of half the levels capped by the tier (tier − 2 for
a Captain's entrants after the first, compared in 64 bits), then on a weapon
event (flag 4) the two hand words the Begin copied from the event's first
entrant (the second with its low word zeroed). The loadout put-on writes the
four hands and the worn list and runs the underwear rule. The tournament
setup hook settles the unentered earlier events before it fills the bracket;
the forfeit prompt's Yes clears every undecided event and refreshes the board;
Continue and the forfeit first resolve every match-copy body under 0.01
health not yet resolved.

## What Changes

- exa_game: `hirelings::match_copy`; `scene_bodies::BodyLook::Company` and
  `put_match_copy` (the copy over the stand-in template's look);
  `arena_live::ArenaScene::match_copy` and `event_hands`, used by the first
  round, the reserve and the bracket's raised fights; `arena_hub_live`
  settles the unentered events and carries the event hands on the launch;
  the board's Forfeit opens the yes/no and its Yes also clears the board's
  entries; `arena_live::sweep_fallen` before Continue and the forfeit.
- `--hub-employee-kit` gives the debug member its kit in hands, outfit and
  six loadouts.

## Impact

Company entrants now fight in their own loadouts (often bare-handed until
armed in the loadout form, as in the original). The manager keeps the
template (the career does not keep his kit). exanima01 and the arena duel
lockstep traces are identical to round 141.
