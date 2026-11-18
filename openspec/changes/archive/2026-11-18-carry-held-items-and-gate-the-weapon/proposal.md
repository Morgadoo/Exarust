## Why

Roadmap task B4, lane `body2`: the open rows of the triage's port lanes L04
(combat and fight AI), L01 (motile body and balance) and L09a (characters
and animation). A downed body's route stop left its run bit and free walk
set; an NPC standing off the navigation grid's inner cells stayed in the
scene stamping nothing; a placement moved the holder's body but left its
held items where they were until the re-place (and for good where no
re-place follows); a touch on a sleeping holder reset only its own
contacts; and the attack's weapon-in-hand gate had no caller.

## Evidence

REA (Exanima.exe 0.9.5): the AI tick's down branch calls the route stop
(the run bit and the free walk clear on the body); the footprint stamp's
overflow branch (cell within five cells of the grid edge: the body flagged
once, unlinked from its room, its node queued for deletion, the overflow
logged once a run); the body placement's recursion into the bodies linked
to the placed one; the touch's walk of the body tree (a sleeping body drops
its contacts); the swing routine's gate after the lead-in (flag 8, the
primary weapon's grip point under its hand against its body, 50·scale cm
and a 0.5 axis dot, a failed gate takes 0.1 off the rate and returns).

## What changes

- `exa_ai::brain::ai_tick` runs the route stop's body side on a downed
  body (`downed_route_stop`).
- `exa_game::footprints::keep_footprints` removes an NPC whose stamp
  overflows the grid (`overflows`).
- `exa_physics::grip::CarryHeld` and `grip_solve::carry_held`: the
  placements carry the held items with their hands before the re-place.
- `Ragdoll::touch` marks the reset; `share_holder_sleep` resets the
  holder's sleeping items' contacts.
- `exa_combat::swing::run_swings` runs `weapon_gate` with `item_in_hand`
  (flag `WEAPON_IN_HAND`).
