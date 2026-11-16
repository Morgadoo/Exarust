## Why

Roadmap task B4 (lane `bodybal`, the triage's port lane L01 "motile body &
balance"): the open TMotileDynamics, TMotilePart, TTetraRBD and TSimSector
rows of the function ledger that a player feels. The rows were re-checked
against the assembly first; several were stale and are re-rated, and the
round 285 follow-ups on the Height Query were settled.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The leg routine halves the standing thigh factor and quarters the leg
  limit when the character's golem-suit byte is set. The character init
  sets it for a golem race (the race word's golem bit, not the skeleton or
  the wraith); the armour pass's golem-suit registration sets it from the
  suit (odd kind and a charge byte under 0xff); the suit's worn tick turns
  it on while the charge still rises and off once it does not.
- The step's end, when the balance-step byte is set (read before it
  clears), stamps the stand-still clock with the game clock plus 100 ms.
- The class set-up computes the per-tick factors from the rate 83: the
  servo multiplies the motion it measures by rate x 0.0125 (1.0375) before
  dividing by the squared gap, and the step's approach multiplies its
  closing speed by 80/rate (0.963855).
- The off-balance arm setter call comes only from the posture block's own
  test: last tick's balance under 0.6 (double) or the getting-up condition.
- A dormant body lying still sets tone and energy to 0 and runs the full
  capture. The get-up compares the rise direction with the heading (the
  body's locomotion heading), and its compare of the clip's progress with
  0.25 returns on the unordered branch (a NaN keeps the clip).
- The quaternion product is the Hamilton product with its operands swapped
  (the first argument's turn, then the second's); the living and pose-only
  samplers call it with the clip turn first and the femur posture second.
- A held item hangs in its holder's body tree, which shares one sleep
  countdown from its root: the living limb step wakes the tree to 256, a
  dormant holder's take counts the tree down, and a child body raises its
  parent's countdown to its own.
- The body tick's jolt calls the behaviour's soft stop for a moving body,
  before the behaviour tick.
- The swept nearest test takes only hits at 0 < t, in both its forms; the
  down body's foot probe starts at its contact (its raise is 0 while down),
  so a contact on the floor misses it in the original too. The Height
  Query's sphere mode runs in every game level: the scene word's grid bit
  is set by the grid chunk reader unconditionally, the room builder and
  the grid resize, and never cleared.

## What Changes

- `exa_physics::body::Drive::golem_powered`, set by `Ragdoll::with_race`
  and by `exa_game::item_effects` (`WornEffects::pending_power`, the worn
  tick's on and off); the posture stage carries it into the bearing.
- `exa_physics::body_balance::end_step` stamps `still_plus`.
- `exa_physics::body_feet::servo` and `drive_contact` take the factors.
- `exa_physics::body_muscle::tune_muscles` reads the bearing's off balance
  alone.
- `exa_game::balance`: the dormant lying capture, `get_up_heading`, the
  NaN progress keep.
- `exa_game::stages::sampler::femur_posture` multiplies in the original's
  order with `engine_math::quaternion_product`.
- `exa_physics::grip_solve::share_holder_sleep` replaces the grip mark's
  every-tick wake; `Ragdoll::follow_holder`.
- `exa_ai::route_walk::soft_stop_jolted_routes`, before the AI states.
