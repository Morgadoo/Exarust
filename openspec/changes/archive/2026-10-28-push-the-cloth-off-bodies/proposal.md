## Why

Round 26's GPU counters (`EXA_CLOTH_CONTACTS=1 EXA_DEBUG_CLOTH=1 play exanima01`) showed 5 of exanima01's 14 banners awake forever at countdown 32, each offered 6678 body parts and keeping 250 to 284 body spheres, with about 200 000 level-triangle tests a tick. The cloth stage cost 12.7 ms a tick. The original's assembly shows three ways Exarust's cloth departed from it:

- The original never pushes cloth particles out of body spheres one by one. Its body-contact call pushes cloth triangles off each sphere into a push buffer. The buffer never adds the same direction twice. An awake cloth applies it once after the body stage, stops or shortens each vertex's travel across its push, and makes three more constraint passes that hold the pushed vertices on their push planes. Exarust pushed every particle out of every sphere in turn, after its only constraint pass. Overlapping sphere sets and pins fought every tick, so a banner resting against sleeping props never stopped.
- After a push, the cloth and the body trade sleep countdowns: the lower takes the higher. A sleeping body's countdown is 0, so it never wakes or keeps a cloth awake. A moving one hands the cloth its own countdown. A count goes up only while the cloth moves, and the cap of 32 applies only then. At the tick's end, an awake cloth whose count is under 2 falls asleep.
- The level's mesh queries hand the push callback only the triangles their tree finds around the particle's sphere. Exarust tested every gathered triangle for every particle.

## What Changes

- New `exa_physics::cloth_body`:
  - each cloth triangle's plane and swept record (the smallest sphere about it at the last tick and now);
  - the triangle walk with the cloth's swept query (`cloth::swept_hit`);
  - the take test, the closest point with its weights, and the push buffer's add;
  - the countdown trade, the push pass, and the plane hold in the constraint pass.
- `cloth::step`: the body stage, the push pass and three constraint passes for an awake cloth. Only a count that went up is capped. The tick's end puts a count under 2 to sleep. The per-particle sphere push and the swept-sphere vertex push are gone. So is the misread pressure-pass record (`apply_cloth_pressure`), which the live push pass replaces.
- `sector::ObjectContacts::gather_bodies` hands over `ClothBody`s: each part's spheres, padded, last first, with its body's countdown. A standing sphere's box test uses where the last tick left it.
- `cloth_level::touch_level`: a per-piece `TriangleGrid` gives each particle the triangles near its sphere or sweep, in list order.
- `exa_world::cloth`: passes the body countdowns. The debug counters gain `bodies` (parts handed over) and count the body stage's triangle tests. The contacts stay opt-in until the GPU check below passes.

Phase 3 (bodies) in docs/PLAN.md.

## Evidence

REA assembly of the cloth's VMT slots:

- the body contact, its triangle bound and plane builders, and its swept triangle query;
- the post-body push pass and its constraint pass;
- the integrate slot's countdown and the end-of-tick slot;
- the sector's object stage, with the object masks, the order of the body, push and static calls, and the end-of-tick call.

## Out of scope

- The body's share of a contact: 0.002, 0.01 or 0.015 of the summed push as an impulse, the friction on the body's velocity, the rustle energy, and waking a sleeping body to the cloth's countdown. These need a mutable body in the cloth stage.
- The body-box triangle cull. This port culls against the sphere about the body's sweeps instead.
- The pin rule for unskinned cloth (a labelled guess).
