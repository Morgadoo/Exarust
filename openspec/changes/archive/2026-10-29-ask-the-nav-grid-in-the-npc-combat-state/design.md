## Context

`exa_ai::combat` reaches the world through its `Field` trait. Its ground cost, line walk, snap and sight defaulted to the abstract pathfinder's answers. `exa_world::nav` already has the level pathfinder's queries (`cost_at`, `line_walk`, `obstacle`) in metres, reviewed against the original, and `NavFootprints` keeps every character's footprint on the grid's penalty bytes each fixed tick. The relevant format page is `docs/NAVIGATION.md` (Queries, Footprints).

## Decisions

- **Which query answers which call.** The pathfinder's slot numbering (movement notes §10.1) maps the combat state's calls: the clamped line walk → `nav::line_walk`, the obstacle within 40 cm → `nav::obstacle`, the cost sample → `nav::cost_at`, and the snap → never overridden, so the abstract no-op. The combatant's cost routine reads the sample's first word as the clearance and its second as the penalty and computes `b·40 + ((a² + a)·120 + 1)` (`combatant::candidate_cost`).
- **Units.** The combat state works in centimetres and the grid in metres; `ground` converts at its edges. A line walk that is not stopped leaves the asked point untouched, so no metre round trip creeps into an unclamped place; a stopped one keeps the asked height, as the grid's clamp does.
- **The own footprint.** The original's AI tick lifts its footprint before its state and stamps it after. Without that the fighter's own 63/32/16 penalties would make where it stands dearer than any step. The duellist lifts the footprint `NavFootprints` recorded for it before its combat ticks and stamps it at the same point after them, a wrapping add after a wrapping subtract, so the grid ends the frame as it started.
- **No grid.** Every answer falls back to the abstract pathfinder's: cost 1, no clamp, no obstacle, as before.

## Open questions

- Which word of the cost sample is the clearance: taken from the reviewed `exa_world::nav` port of the level pathfinder's cost sample (answered there by REA on the level pathfinder's sample routine).
- The stab head tracking's aim point and the held-weapon pushes that read it: answered when the handling pushes are wired (REA on the body's handling routine).

## Risks

The duellist's places now differ near walls and characters: a place 30 cm from the player's standing footprint costs up to about 6 more, so a duellist may hold off a step further from a standing player than before. The fight should still close and strike; checked on the GPU arena opponent run.
