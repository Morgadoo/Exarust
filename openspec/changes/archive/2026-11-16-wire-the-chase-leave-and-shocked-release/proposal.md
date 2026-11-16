## Why

Roadmap task B4 (lane `combatai`, the triage's lane L04 "combat & fight
AI"). The chase's leave was never reached live: the duellist marked every
chase fresh, so a fighter that lost sight of its foe kept re-planning a
route instead of leaving combat, and the leave kept its level. A cast
started while parrying left the parry on, so the next parry tick could
start the parry clip over the cast. A released race shocked while
attacking kept its swing, and a dying released race still took the hit
and moved its shock clock.

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly:

- The chase: before its branch it stores the body's target as the body
  in the way; not following a route, it holds on the integration's fresh
  mark (written each AI tick from the line-of-sight test of the body's
  target, set for a focus without a target, and by the set-target),
  otherwise it clears the body's target and focus, the AI's focus object
  and its level and returns to the idle state.
- The cast start: with a motion and the parrying byte set, the clinch
  clear runs before the motion starts (blend 0).
- The shock reaction: a release race that is collapsing or dead returns
  before the hit sound and the clock; the shocked release, in condition 2,
  writes the swing's stall 1.0 (the float constant read back from the
  image), its strength 0, flags |= 6 and its end at now + 600 ms.
- The parry trigger's grip test returns false only on the attacker's
  cleave bit, which the trigger has refused first: its failed-grip branch
  (clear, then the stance by energy at blend 20) is unreachable.

## What changes

- `exa_ai`: the duellist's chase reads the brain's `integration.fresh`
  (a brainless fighter: always fresh), its leave drops the level, and
  each chase tick writes the driver's body in the way.
- `exa_combat::parry::clear_for_cast`, called by
  `exa_game::thaum::cast::play_cast_motions` when a cast motion starts.
- `exa_game::thaum::live`: `shock_reaction` refuses a dying released
  race; `shock_fighters` resets the swing of a released attacker.

## Out of scope

The body heading that trails the look (the look snap), NPC head tracking
during the stab, the search's perceived goal entity (F389), the living
sampler's queued-clip switch, the weapon-in-hand gate, the blade paint's
-2 mark: each needs a body or perception path Exarust's NPCs do not have
yet (see the lane's ledger rows).
