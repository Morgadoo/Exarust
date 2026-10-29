## Context

The lane ported from the pseudo-code and the report's own reading of field meanings. The review read each routine's assembly.

## Decisions

- **Footprint.** The gate's first flag is set only by the route planner when it starts following a route and cleared when the route stops; it is not the combat state. Exarust's game footprint system already stamps every NPC at its feet each fixed tick, and the brain's extra stamp used the fighter entity's origin, so the brain's stamps are removed and the gate is kept as a tested function.
- **Familiarity.** The relation writer looks the partner up (in the flagged or plain relation list) and adds the drift to its base only when found.
- **Set target.** The routine subtracts the chest from the estimate, takes the flat length (x² + z², square root), multiplies the offset by its reciprocal and writes the yaw to the aim. A zero length keeps the previous direction [Exarust's guard: the original divides by zero].
- **Stance.** The set-stance helper's motion 3 is the Defence stance; motion 4 is the combat idle.
- **Leave.** The target pass calls the leave-combat routine, which writes the look and aim, zeroes the heading and draws the sway 500 ms out; the brain's out-of-combat turn rewrites heading and eased offset each tick, so the existing leave look stands in for those writes.

## Risks

A duel now starts with the aim on the bearing to the foe, so the first combat tick's re-aim compare starts from it. An NPC entering a fight blends straight to Defence again.
