## Why

Roadmap task B4 with B1 (lane `tetrarbd`, docs/roadmap/phase-b-close.md):
the open TTetraRBD, TRBDDoor and RF_Dynamics_Volumes rows of the function
ledger. Three differences from the original were found and closed: the
hinged door's drive was pushed before the leaf's next integration (which
carried it as velocity at once); a rigid body's impact class bit was taken
as always set; and the pair stage's immediate push summed its strain
length in another order than the original's length routine.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The hinged door step measures the leaf (handle, stop point, angle), then
  calls the inherited Verlet step (the integration), then computes the
  drive target from the measured stop point and pushes it by the stop
  link's weights (its raw-weight push routine) after the integration. A
  sliding leaf calls the integration first and runs its sliding rules after.
- The sector registration of a rigid body calls the mobility and impact
  class routine before the class's init slot: that routine takes the
  body's collision sound list, else its node's first mesh material's, and
  sets the class bit 0x40000 only when the list has an entry. The door init
  then gives a leaf whose list is empty its slam set, and sets the bit when
  that has an entry. No other writer sets the bit on a plain rigid body
  except the force effects.
- The engine's vector length routine squares the packed lanes, adds the
  high pair onto the low one and then the second lane: (x² + z²) + y².

## What Changes

- `exa_physics::body::Ragdoll::push_after_integration` queues a push that
  the body's next integration step applies right after integrating; the
  hinged door's drive uses it (`DoorBody::drive`). A sleeping tick drops it.
- `exa_audio::bodies::impact_class` clears `Part::reacts` on a newly
  registered body's parts whose collision set has no shipped variant
  (`BodySounds::impact_set`); a door leaf's `BodySounds::fallback` names its
  record's slam set, which its impacts ring when its own set is silent.
- `exa_physics::body_pairs::shove` sums its strain length (x² + z²) + y².

## Impact

Simulation traces change for door leaves under a drive (props knocked
over, doors, thrown items scenes); props and items whose material has no
collision sound no longer make level impacts. No GPU run was possible (the
card is on hold); the tests are the evidence.
