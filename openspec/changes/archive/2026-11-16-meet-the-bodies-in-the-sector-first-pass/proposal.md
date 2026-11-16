## Why

Roadmap tasks B1 and B4 (lane `simprepass`, docs/roadmap/phase-b-close.md):
the grid sector step's first pass, before the character ticks, runs the
simulated objects whose class has the pre-pass bit against the bodies of
their nine cells. Exarust had no such pass: the force blast and the force
wave struck every body near their bounds in their own tick, after the
solver. The particle systems' body pass (their body-contact slot, the
contact walk and the push-out callback) was ported but uncalled, and
its contact walk differed from the original: it tested the particle
against the body's origin instead of its node's bound, had no box test,
no mesh query, guarded a zero life and took the life difference in
single precision.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The first pass: the object list in order, class bit 0x40000 set and no
  state bit of 0x15; the nine cells around the object's cell (the grid's
  offsets), each cell's bodies; a body passes when its bound sphere is not
  apart from the object's (`0x100077540`), the object's centre squared
  distance outside the body's box is under the radius squared (NaN goes
  on) and the body is not disabled (state 0x800); then the object's body
  slot.
- The bit is set only by the blast's release and the wave's throw (the
  wave shares the blast's class and body slot). The blast and wave carry
  the static and object-pair bits (0x10000, 0x80000), not the solver's
  body bit (0x20000). The fire presets' and the portal's 0x40000 writes
  are on their light sources (`TLightSource`), and the displacement
  shell's on its geometry node (`TShellEffect`), not on sim objects: the
  earlier reading (round 265) that the shell and the particle systems take
  the first pass was wrong.
- Every particle system's constructor sets the class word 0xa0020000: the
  solver's body bit. The particles meet the bodies in the solver's object
  stage after body-body detection (body state mask 0x804, 0x805 for an
  asleep object), not in the first pass. The fire presets clear the bit
  on their second system.
- The particles' body slot: skips the body whose node is the system's
  node's parent, zeroes the count, takes the body's matrix, walks the live
  ring in one range or two, then calls the body's dose slot with
  `(float)count × dose` (the template's dose). The walk: rate
  `growth / (float)start_life`; per live particle (life ≥ 1) the transpose
  product into the frame, radius `(float)(i64 start − i64 life) × rate`,
  `(r + R)² < d²` and `r² < box²` skip (NaN goes on), the node mesh's
  two-sided sphere query (slot +0x3a0) with the push-out callback
  (`d = c − q`, `d² ≥ r²` or `d² < ε` leaves it, NaN goes on, else
  `c += d·(r/√d² − 1)`), back to the world by the row sum
  `(x·X + y·Y) + (z·Z + T)`, velocity `+= (old − new)·−0.98`, count + 1,
  life − 1.
- The dose slots: the rigid body's is the base no-op, the door's returns
  its argument; the motile part's adds the dose to its character's burn
  strength when its skin bone slot (+0x468, bone − 1) is above zero.

## What changes

- `exa_game::thaum::pre_pass::meet_bodies_first` in `BodyPreStep`: the
  thrown blasts and waves in entity order, the nine cells' parts
  (`exa_world::cloth_parts`), the sector's gates and the disabled bit, the
  blast's part contact. The blast's and the wave's own ticks keep their
  level and cloth contacts and no longer strike bodies.
- `exa_world::particles`: `Emitter::body_pass` and `Emitter::contact` as
  the assembly, `particle_push_out`, the `BodyContact` trait as the
  original's body and node, `Params::dose`.
- `exa_world::particle_stage`: `PartContact` (a part's frame in cm with
  the exact transforms, its node bound, the two-sided mesh query) and the
  solver stage `ParticleSolve` for staged emitters.
- `exa_physics`: `engine_math::point_into_frame`, `point_out_of_frame`;
  `mesh_tree::two_sided_contacts` (shared with the blast's part contact).
- `exa_combat`: `burn::take_particle_doses` after the solver; fighters
  carry a `Burn`, so the upkeep's burn runs for every character.

Open: the fires' emitters still step once a frame in the frame's world,
so no emitter is staged into the solver in the running game.
