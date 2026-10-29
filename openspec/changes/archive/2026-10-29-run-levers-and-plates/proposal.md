# Proposal

## Why

docs/PLAN.md, Phase 4 (the player's controls and behaviour: the hand on
the world, SG_PlayerControls unit routines) and the mechanisms of Phase 3/4
(TRBRotator, TRBPushSurface, TRBDDoor's sliding leaf, RF_Dynamics_Game's
signal dispatch). Round 31 made hinged door leaves bodies; exanima01's 40
levers, 12 push surfaces and 37 sliding door leaves stayed static. Their
rules (`exa_physics::lever`, `plate`, `door`'s sliding step, `mechanism`'s
dispatch) were tested, but nothing measured an arm or a plate, the wheel
drag was only recorded, and no signal reached a target.

## What Changes

- A lever's arm registers as a hung body with the lever's set-up (mobility
  1, the mechanism environment of no gravity and 0.001 kept a second, the
  lever frame from the axle and the arm link's tied point, the detents at
  the ends of the swing, a saved return re-armed) and runs the lever's step
  after the solver, the drive moving the arm point onto the arm's radius.
- A push surface registers on its slider links with its set-up (mobility
  0.3 or 0.03, the mechanism environment) and runs the plate step on the
  links' travel fractions.
- A lever's or plate's report sends a mechanism signal; the level routes
  it by the target's records to the door bodies or the push surface under
  the target's node.
- The hand names a lever's arm the wheel: the press holds the lever and
  keeps the arm-to-cursor angle on the plane across the axle; each frame
  sets the lever's hand angle (first or second grip by the view, clamped
  to the swing), lets go out of 300 cm, and the release lets go.
- The double-click interaction pushes a push surface whose flag 4 is clear.
- A root body's slider link keeps its travel as slack (it was converted as
  a child's, 289 m for a 1.7 m portcullis rail).
- A sliding door's leaf can register with the door's sliding step behind
  `--sliding-doors`; `--static-mechanisms` keeps levers and plates static.

## Evidence

REA (Ghidra) on Exanima.exe 0.9.5: the lever init, step, record read,
timed return and tick loop; the push-surface init and step; the door init's
sliding branch; the signal dispatch's class 5 branch; the controls' object
interaction (class 5, flag 4); the hand's wheel press and drag handler,
with its instructions for the grip test and clamps (double compares, NaN
paths); the vector helpers (plane removal, cross product, row and transposed
products, exact and approximate normalisations). `exa-tool mechanisms
exanima01` and `exa-tool bodies` on `lever a01`, `portcullis uwa`,
`pressplateuwa1`, `button a02`. Real-data tests on `exanima01` (40 levers
and 12 plates rest; a turned lever signals its portcullis; the 37 sliding
leaves read fraction 0 on level rails and creep at rest).

## Out of scope

Sliding doors live by default (they creep and their lift is not
reproduced); relays and deferred fire and light jobs; a kind 6 lever's light
fade; mechanism sounds; the sliding-door drag; the navigation passage
report.
