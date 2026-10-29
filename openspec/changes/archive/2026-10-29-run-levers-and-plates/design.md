# Design

Extends `docs/PLAYER.md` (The hand, and a new section, Levers, push
surfaces and sliding doors), `docs/formats/RFC.md` (registration counts and
a root slider's slack) and the mechanisms sections of the `exa_physics` and
`exa_world` READMEs.

## Bodies and glue

Each mechanism registers through the hung-prop path (`exa_world::bodies`)
with a pending component on its root (`PendingLever`, `PendingPlate`,
`PendingSlide`), turned into its glue once the `Ragdoll` exists, as
`DoorBody` is. The glue runs after the solver's tick (the original runs it
in the body's step before the link passes; the links meet it a tick
later).

The mechanism environment is a per-body override on `Ragdoll`: no gravity
shares and a fixed per-tick keep of 0.001^(1/83); the speed bands that
drive the sleep countdown are the default's (the original's environment
record keeps the same 40/20 cm/s bands).

## The lever frame

The frame's up row is the axle (the last pin farther than 1 cm, exactly
normalised) turned by the node's own rotation and negated; its forward row
is the arm link's tied point less the node's position, its part along up
removed (`a − (a·b)b`, read in the helper the init calls, which is not a
cross product), normalised; its side row up × forward. The tied point thus
lies on the forward row, so the first detent is 0 − α, which the
pre-existing `lever::shape` had guessed. The arm's angle uses the node's
world position at set-up as its origin, as the original copies it once.

## Signals

`MechanismSignal { target, source, value }` is a Bevy message written by
the lever and plate steps. `exa_world::mechanisms::route_signals` finds the
placed object whose id is the target (a linear scan; signals are rare),
dispatches by its records, and applies a door order through the existing
`open_doors`/`close_and_lock_doors`/`toggle_doors` on copies of the door
rules of every door body under the node, waking each. Relays and jobs are
logged.

## The wheel

`LeverBody::take_hold` and `turn_to` do the press's and drag handler's
arithmetic in the lever frame; the hand supplies the cursor ray's points on
the two planes (`ray_plane`) and the grip choice (`first_wheel_grip`, a
double compare where a NaN takes the first grip). The previous
`hand::{wheel_turn, wheel_set, uses_first_wheel_grip, wrap_half_turn}` are
removed: `body::lever_body::swing_clamp` is the clamp.

## Sliding doors

The shipped sliding leaves all start at fraction 0 (their slider points on
their anchors) on level rails, and in Exarust's solver they creep up to
2 cm in two seconds without sleeping; the slider law rewrites a command's
fraction from the leaf's place every pass, so an open's jump to 0.6 only
shoves the leaf. Rather than ship a creeping gate, the leaves stay static
by default and register only with `--sliding-doors`.

## Risks

A lever whose frame were wrong would be driven off its detent; the
real-data test checks all 40 rest within 1 cm and on a detent. A plate's
rails now hold their true travel; all 12 rest.
