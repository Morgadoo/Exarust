## Context

The cloth's body contact is a VMT slot the sector's object stage calls once per body that passes the sector's pre-tests. A sleeping object is offered awake bodies only. After the body stage, the sector calls a second slot for an awake object only: the push pass. Then it calls the static slot for every object, and at the tick's end it calls the end-of-tick slot for every object not flagged asleep. Exarust keeps one `cloth::step`, which runs these in the same order.

## Decisions

- **Triangles, not particles.** The original's records are the triangle's smallest sphere (on the longest edge when that edge's square is at least the sum of the others, else circumscribed) at the last tick and now. The walk is the swept query that already exists (`cloth::swept_hit`), with the sphere's coarse radius the padded radius plus a half when it moves, as the instructions build it. The plane and the circumscribed centre use the processor's approximate reciprocal square root and reciprocal, as the original does.
- **Push buffer.** A push that points along the buffered one adds only what goes beyond the longer one's projection. The pass applies the buffer only for an awake cloth (woken this tick included). A sleeping cloth's buffer waits for the next awake pass, as in the original.
- **Countdown trade.** `trade_countdowns` raises the cloth's count to the body's. The body's side (being owed the cloth's count) is returned but not applied: the cloth stage holds the bodies read-only.
- **Representation differences.** The original first culls the cloth's triangles against the body's oriented box. Exarust culls them against the sphere about the body's coarse sweeps, which keeps every triangle the walk's coarse test keeps. The level grid stands in for the mesh's own tree, and grows each particle's box by twice the padding for the pushes taken along the way.
- **Grab.** The hand's grab still wakes the cloth (Exarust's own rule). It now wakes it to at least 2, so the cloth outlasts the tick's end.

## Verification

Tests pin each rule with concrete numbers:

- the smallest-sphere branches;
- the closest point in all six branches;
- the buffer's no-double-count;
- the trade;
- the push pass (stopped, shortened, too small);
- the plane hold;
- a sphere over, under, out of reach and sweeping through a triangle;
- a sleeping cloth over a sleeping body staying asleep and unmoved, then woken to 256 by a moving one;
- a loosely pinned cloth bulged by a sleeping prop's sphere falling asleep;
- the level grid giving a particle only nearby triangles with the same push.

The GPU check is for the integrator: `EXA_CLOTH_CONTACTS=1 EXA_DEBUG_CLOTH=1 play exanima01 --perf-log 5`. A few seconds after the load, the stage line should read awake ≈ 0 and well under 2 ms, with level tests in the low thousands. A banner line with a countdown above 32 and bodies > 0 is touched by a moving body. One stuck at 32 with bodies 0 moves on its own (wind or the level). Only then may `ClothContacts` default to on.
