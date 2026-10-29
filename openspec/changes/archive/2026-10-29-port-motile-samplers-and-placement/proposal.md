## Why

The TMotileDynamics routine map (`docs/re/motiledyn-map.md`) lists the pose-only sampler (spawn and placement), the living sampler's remainder, the two placement routines and the body tick's down-body ground probe among the largest unported pieces of the character body class. This change ports what the assembly states, as tested rules, and fixes the one live difference it found: the living sampler's thorax hunch.

Phase 3 (physical bodies: the character body's sampling and placement) in docs/PLAN.md.

## What Changes

- The samplers' heights: the stand height over the lowest posed foot contact less the sole, the leg reach from the clip contacts, the pose-only sampler's pelvis target (set outright) and the living sampler's (eased 0.1 a tick, the walking ease growing with the run blend, an early get-up clearing it). Tested, not called: Exarust's carry aims the pelvis from the feet.
- The pose-only sampler's key pick (no ease, no clamp, no guard between the keys, the rebase off the capture's key) and its root source for the first three gaits. Tested, not called.
- The living sampler's switch to a queued clip on a new target key. Tested, not called (Exarust starts clips at once).
- The placement routines' arithmetic: the pose lowered by the pelvis's posed height and put on the frame, the foot anchors on the ground and at a matrix, the matrix pins 5 cm lower, the means. Tested, not called.
- The down body's ground probe: its start is the pelvis plus the world origin of the zone its scene node hangs under (zero in Exarust's one world frame), one 10 cm sphere cast 2000 cm down. Tested, not called.
- **Live:** the living sampler's hunch lowers the forward axis of the pinned part's own placement after the torso twist, so the head and arms no longer follow it (feral characters only).
- The routine map's statuses, live column, totals and corrections; the crate READMEs.

## Evidence

REA assembly of `Exanima.exe` and the motiledyn research pack (`context/re/packs/motiledyn/`): the pose-only sampler's bracket and tail, the living sampler's tail and queued-clip branch, the placement routines' part loops, the body tick's probe block and the vector and matrix helpers they call (add, subtract, scale, the relative-transform helper).

## Out of scope

Calling the heights, the placement arithmetic or the probe from the game; the leg snap's frame height write-back; the pose-only sampler's per-part routing as one pass (at a first spawn its twist, posture and hunch are identity); the placement routines' field resets, cloth and attached-body steps; exertion; the muscles-off sampler.
