# Proposal

## Why

The intersection unit's remaining routines are its scene bookkeeping: the light records the environment publishes, the weight sort of a light's link list, and the alias a light finds, makes or refreshes in another sector and spreads along the linked sectors. A pass ported them and re-checked the shadow volume and the volume cut; review against the assembly found that two of its "fixes" reversed NaN branches and a sum order the code already had right, and that the link sort, the alias gate's flag word and the refresh did not follow the instructions.

## What Changes

- New: `PublishedLight`, the sun record (direction times its fourth lane, that lane's root), the reach with its 0.01 dead band (single-precision difference, double compare, NaN publishes, halved while the mode flag is clear) and the sky record.
- New: a light's link list ordered heaviest first by the original's insertion sort (ordered compares: equal weights and NaNs stop the walk).
- New: the alias a light finds or makes in a sector (gated by the option word's bit on the sphere test then the squared box distance; a current stamp prunes, an older one refreshes) and its recursive spread along the sectors' linked lists, run by the activation pass outside grid mode.
- The shadow volume returns the original's side count, the planes' last index.
- The volume cut's cap closes only below 1e-4 (a NaN distance turns and chains), and a silhouette edge sums (x² + y²) + z² and keeps a NaN length, as before this round.

## Capabilities

### Modified Capabilities

- `intersection-tests`: the volume cut's NaN branch and the shadow volume's returned count.
- `scene-nodes`: light activation spreads the alias; link order, alias spread and published light records added.

## Impact

- `exa_physics::intersect`, `exa_world::scene_nodes`, new `exa_world::published_light`; both crate READMEs.
- Evidence: the unit's routines in the executable's assembly (the setters' dead band and scale helper, the sort's comiss with a parity exit, the alias routine's option-word test, sphere and box gates and refresh call, the spread's busy byte, the activation pass's call into the spread, the cut's comisd against 1e-4 and the edge's mulss/addss order).
- Out of scope: wiring the published records into the renderer (the running sun goes through `scene_env`), populating sectors' linked lists from the room flood (so the running game spreads nothing yet), the link weights' writers outside this unit, the unit's RTL destructor, and the processor's approximate reciprocal and root.

## Phase

Phase 3, bodies and physics: the shared intersection maths and the scene bookkeeping the lights build on (docs/PLAN.md).
