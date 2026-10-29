# Proposal

## Why

docs/PLAN.md, Phase 4 (the player's controls and behaviour: the hand on
the world, SG_PlayerControls unit routines) and the mechanisms of Phase 3/4
(TRBDDoor, the door bodies). Doors were static level geometry: the door
rules (`exa_physics::door`) were tested but nothing measured a leaf or drove
it, and the hand only recorded a door drag. A round-27 pass left them
unwired, worried that door models carry about 1 900 collision spheres, that
the hand's 0.003 would barely move a leaf under the horizontal-drag law, and
that no door state held a leaf shut. Read against the instructions, the
hand's door link is not a horizontal drag: the press passes slack 1.0 and
strength 0.003 and sets the link's flags to the iteration limit alone, so
the plain slack law pulls the leaf; the door's own drive holds a shut leaf;
and the original never meets a door's spheres from a character (the more
mobile side brings its spheres against the other's mesh).

## What Changes

- A placed door's hinged leaves register as hung bodies (their pin, swing
  stop and origin pivot) with the door's set-up: mobility 0.05, the door
  frame from the hinge axis and the stop's tied point, the handle point,
  the placement's stored word, a sibling leaf as the partner with pair
  layer bits `0x3000`.
- Each tick an awake leaf measures its handle energy, stop travel and
  angle, runs the door's step and is driven by the stop point onto the
  driven angle, so a woken shut leaf stays shut.
- The hand names a leaf a hinge; a latched leaf rattles and refuses; a
  drag adds the door link (slack 1 cm, strength 0.003, three passes) at the
  point 100 cm from the door's origin toward the hit, pulls it 100 cm from
  that origin toward the cursor each frame, disables it while the hip is
  more than 350 cm from the door, and removes it on any release.
- The pair stage's sphere fallback buckets a part's spheres when it has 64
  or more, with the plain loop's results.
- `--static-doors` keeps doors static (Exarust's debug option).

## Evidence

REA (Ghidra) on Exanima.exe 0.9.5: the engine's link-adding routine (slack
squared, strength), the hand's door press and drag handler, the door init
and hinged step (frame, angle, drive), the vector helpers they call (row
versus transposed products), the placed-object reader (the stored word),
read in the instructions; `exa-tool bodies` on `door uw01a` (1 899 spheres,
a 243 cm pin and a 134 cm stop); a real-data test on `exanima01` (109
leaves).

## Out of scope

Sliding doors, levers (the wheel drag) and push surfaces as bodies; the
door's sounds; the navigation passage report; the last-pusher stamp; the
door's mesh as the pair's mesh side.
