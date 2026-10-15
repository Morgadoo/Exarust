## Why

Exanima's characters lose their footing: a heavy enough blow on a dazed body
topples it, and a fallen character lies there for a moment and then gets up.
Exarust bodies only staggered and stayed on their feet.

## What Changes

- Balance per the original's rules: the centre of mass against the line
  between the feet, an energy-scaled threshold, a stability⁸ score with a
  counter, and recovery.
- A body that loses its balance falls. It loses its controller's support,
  its tone drops to 0.5, it plays the fall clip into lying, and its limbs
  are held relative to its own pelvis. Player and AI stop steering it.
- After settling and recovering, it gets up with whichever get-up clip lies
  the way the body does. The controller is moved and turned under the body
  first, and balance rises as it stands.
- `exa-tool rsq` lists the actions. The viewer's `--shove` deals a body
  blow from the front.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: balance, falls and getting up.

## Impact

- `exa_physics::body` (support, centre of mass, `Fallen`), the new
  `exa_game::balance`, `exa_game::player`, `exa_ai`, the viewer, `exa_tool`,
  and `docs/formats/RSQ.md`.
