# Proposal

## Why

The original's behaviour base (`TCombatant`) holds what every character
behaviour shares: the contact list, the perception of one other character
(range, closeness, noticeability, view, the attitude's group blend, the
threat estimate and the crowd penalty), the mind-power overrides of an
attitude and of trust, the script's concern call, the published attack arc
and time to impact, the combat position score, the queued combat step and
its two clears, the nav footprint, and the reference bookkeeping when a
character is struck, dies or is swapped. Exarust had its head tracking and
part of the position score. A first implementation of the rest drove the
duellist, but a review against the instructions found many rules wrong:
the position score cleared the danger outside a fight and read the
opponent's facing and feral level where the original reads the observer's;
its approach fraction was inverted inside the strike ideal; perception
invented a facing and a bearing term, raised the noticed level where the
original caps it, weighted third parties to the sixth power instead of the
eighth and wrote the third-party raise to the threat instead of the
concern target; trust kept the frenzy branch and dropped suspicion; the
footprint used a floor and moved both axes; and many steps lost their
double-precision arithmetic. This change records the rules as the original
has them.

## What Changes

- The perception pass of one character (`exa_ai::combatant::perceive`)
  writing the observer's contact record, with the effective attitude,
  trust, the group blend, the threat estimate, the crowd penalty carried
  across passes, the third-party concern target, the quick-swing event
  and the familiarity drift.
- The script's concern call with its two event bits.
- The combat position score (`exa_ai::place`) corrected: danger in and out
  of a fight, the approach peaking at the strike ideal, the observer's
  heading, attack state and feral level, the thrust's approach scale past
  the gate, double-precision thresholds, sums, wraps and danger ramp.
- The queued combat step's clears, the move-point clamp straight ahead,
  the nav footprint (round to even, per axis), the hit rule with the
  attacker's mark, forget, replace and reset.
- The duellist (`play --opponent`) perceives its target through the pass
  each frame and scores its spacing with the corrected position score.

## Capabilities

### New Capabilities

- `npc-combatant`: the behaviour base's references, queued step, footprint
  and hit rule.

### Modified Capabilities

- `npc-perception`: the perception pass, the mind-power overrides, concern.
- `npc-attack`: the corrected position score.

## Impact

- `exa_ai::combatant`, `exa_ai::place`, the duellist in `exa_ai`; the
  `exa_ai` README.
- Evidence: REA/Ghidra listings and pseudo-code of the behaviour base's 30
  functions in `Exanima.exe` 0.9.5 (the perception pass, the two attitude
  wrappers, the concern call and the entity setter it calls, the position
  score, the attack arc, time to impact, the is-attacking test, the arc
  test, the step cost, the step clears, the footprint stamp and removal,
  the point of interest, the look direction, forget, replace, removal,
  reset, the rating refresh and the hit rule), read at the instruction
  level for every float compare, sum order and precision. No game data is
  read.
- Out of scope: the role database feeding the pass at run time (raw
  attitude, tolerance and trust are inputs), the encounter scripts and the
  familiarity write, the mind-power calls the pass and a hit make, the
  scene's forget broadcast on removal, the pathfinder's footprint insert
  and the room list, the combat state's candidate search (Exarust keeps an
  interim footwork choice), target selection and morale.

## Phase

Phase 6 (AI), the combat AI that drives the same character controls as
the player; it serves milestone M4 "first duel" through the duellist.
