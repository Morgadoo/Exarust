## Why

Roadmap task B4 (lane `doors`, docs/roadmap/phase-b-close.md): the open
TRBDDoor rows of the function ledger and finding F471. Exarust left every
sliding door static unless `--sliding-doors` was passed, because the
leaves seemed to need a lift that was never traced; the original builds a
door body for every door record. The sliding step also ran after the
whole solver, where the original runs it right after the leaf's
integration.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The door step (class slot 15) integrates a sliding leaf first (the
  inherited Verlet step), counts down a signalled wait, measures the
  handle (`s + (v − s)·½`, energy (x² + y²) + z²) and then runs the
  sliding step; a hinged leaf measures, integrates and then drives.
- The sliding step: the frozen leaf's parked fraction; the held leaf's
  drag link anchored at the drag target within 60 cm of the link's
  point (scaled only when the squared length is ordered and above 3600);
  the creak; the bottom knock; the unlock timer; the open (mode 4, the
  body woken to 256 each tick) and close (mode 8) commands; the shut test
  and slam; the held pull.
- exanima01's 37 sliding leaves are all `PortCullis UWA`: three slider
  links along the model's −x, 170 cm of travel, placed with their slider
  points on their anchors (fraction 0, the open end) and the model's
  portcullis 170 cm aside in its wall. The rails lie level: a close slides
  the gate into its frame; no lift is involved.
- The motor's toggle and open compares (COMISS/COMISD with JP to the
  arrived branch) arrive when unordered.
- The clone (class slot 13) copies the init's derived fields, all of which
  the init re-derives on the registration that follows (the copied 12 bytes
  after the door frame are never read). The destructor (slot 0) frees the
  creak voice.

## What Changes

- `exa_world::bodies::MechanismBodies::default()` registers sliding
  leaves; `--static-sliding-doors` keeps them static (replaces
  `--sliding-doors`).
- `exa_physics::body::slide_body::SlideStage`: a solver stage that steps
  each awake sliding leaf right after the integration
  (`SolverStage::integrated`); an open command wakes the body.
- `SlideBody::take_hold`, `pull_to`, `let_go`: the hand's drag of a
  sliding leaf (wired in `exa_game::hand`).
- The hinged and sliding ticks smooth the handle as `s + (v − s)·½` and
  sum the energy (x² + y²) + z²; the motor's compares arrive when
  unordered; `door::slide_hand` follows the original's compare.

## Impact

exanima01's portcullises become bodies: the player and characters meet
them as bodies, their room links follow their pair flag, the router's
door orders move them, and a woken leaf creeps at most about 2 cm along
its rail before it sleeps. Simulation traces change for every scene with
a portcullis (exanima01). No GPU run was possible (the card is on hold);
the tests are the evidence.
