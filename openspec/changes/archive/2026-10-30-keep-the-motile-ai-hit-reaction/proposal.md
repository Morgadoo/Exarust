# Proposal

## Why

Phase 6 (AI) of docs/PLAN.md: the motileai8 DeepSeek lane's pass on
TMotileAI. The original raises a combatant's concern for whoever hit it,
weighted by how little the attacker seems taken up with other nearby
bodies; the AI tick times out its grabbed flag; and the investigation's
goal callback sends the walker to the pathfinder's nearest walkable point.
Exarust had none of these.

Evidence: the original's hit reaction, its nearby-body threat estimate,
the grabbed-flag step at the head of the AI tick, the investigation goal
callback and the alert look-around, read in REA (Ghidra listing) and the
objdump listing (the vector subtraction helper is body aim point minus
attacker position).

## What Changes

- `exa_ai::combatant`: `threat_from_nearby` (1 less the largest share
  `dot / d²` of the attacker's look a nearby body takes up, within the
  attacker's radius plus 100 cm, dead bodies and the attacker skipped),
  `hit_concern` (threat + 0.5, +0.5 for kind 1, +0.25 for kind 2, floored
  at 0, scaled by `1 − attitude·0.01` in double, capped at 15; a contact at
  the cap or with a NaN concern is left alone) and `Combatant::react_to_hit`.
- `exa_ai::brain::grab_timeout`: the grabbed condition holds the flag
  unless the state is not the combat state and the body's hook refuses
  event 0x10; otherwise the flag clears once 500 ms have passed since the
  grab (widened sum).
- `exa_ai::lifecycle::investigation_goal`: the walkable point, mode 20,
  radius 100 cm.
- Review: the lane's change of the alert look's 45° gate to the body's
  facing was reverted (the gate reads the AI look field it overwrites);
  the hit reaction's NaN-concern gate was corrected.

## Out of scope

- Wiring: Exarust has no caller delivering a blow to an AI combatant
  (`take_hit` has none either), no grab mechanic on characters and no
  disturbance query (constant 0 in 0.9.5), so these stay tested functions.
- The hit reaction's contact word 3, which has no Exarust field.
