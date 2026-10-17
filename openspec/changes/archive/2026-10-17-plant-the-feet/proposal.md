## Why

Exarust's feet do not stand on the ground: a constant friction (0.5 on
the feet, 2.5 on the shins) is all that keeps them in place, so standing
feet slide under any push. The original keeps a foot contact per leg (its
ankle) with a state, an anchor on the ground and a grip: its sampler and
footwork plant the feet, stand them on their stance spots and step a foot
whose spot drifted; its feet routine sets each leg's friction by its
foot's state and each anchor on the ground plus the sole height; and in
every solver iteration a standing foot is gripped at its anchor (a
walking one pulled by a velocity servo, a lifted one only pushed up) and
its toe held over the floor.

Phase 3 (Bodies: skeleton, physics, control), milestone M3 "standing man"
(stands, stumbles when pushed), the master plan's MS2. This is change C12
of the master plan (`context/re/master-plan.md`, lane A, A2), on C6, C8,
C11 and M0's stages.

## What Changes

- `exa_physics`: each leg's foot contact (`body::Leg`: `state`, a
  `body::FootState` of the original's bits, `anchor`, `grip`, `ramp`,
  `approach`, the foot part and its toe) built with the body
  (`body_feet.rs`). In the character tick, before the feet are measured,
  the feet's states (`feet::plant`): planted where they meet the level
  (not walking), not touching (walking, until the gait plants them), none
  touching (lying), on their stance spots (getting up to nine tenths,
  unsupported), the footwork's landing pass and one step at a time to a
  stance spot 5 cm off (Exarust's stand-in for its scores). In the feet
  measurement: friction by state on the lower leg and its foot, the probe
  by the condition (10 cm walking, 5 cm + 10 × an anchored foot's blend
  otherwise), the smoothed ground, the anchor height, the toe's floor, no
  ground (0x30), the stance's level; in the lift, the legs' extension. In
  each iteration, after the lower leg's links, the contact drive (PD grip,
  velocity servo, lifted push, reach within 5 cm, the grip ramps), and
  after the foot's links, the sole push. The carry moves the anchors.
  `Ragdoll::clear_feet`, `Ragdoll::ankle`, `Ragdoll::drawn_foot`,
  `Balance::level`/`extension`/`landings`; `Bearing` gains `walking`,
  `off_feet` and `progress`.
- `exa_game`: the bearing reads walking and off its feet from the
  condition and the clip's progress; the get-up's start clears the feet;
  `--trace` fills the foot contacts' columns and adds the ankles; the body
  overlay draws the anchors and the readout shows each foot's state and
  grip.

Evidence:
- `Exanima.exe` (objdump at the routines' starts; constants read from the
  PE's `.rdata`/`.data`), with the Ghidra decompiles under `context/re`:
  the limb link iteration's foot contact and sole steps, its velocity servo
  and PD grip helpers, the feet routine, the balance routine's footwork
  (the bad, unsupported, down and stepping branches, the landing pass), the
  standing sampler's plant and its queued clip switch, the contact
  clearer's five callers, the muscle setup's toe point and foot friction,
  the condition word's values (condition 1 is walking). Findings:
  `context/re/body-feet.md` §18.
- Unit tests pinned to the rules' numbers; light runs on the RTX 4000
  (`play arena`: standing, quarter turns, walking and stopping, `--shove 60`,
  `--opponent`).

Out of scope:
- the footwork's stance scores, timers, recovery-step trigger, step
  target and step end (C15); the walking gait's lifts and plants, the swing
  flag, the closing step and the gait-weighted ground (M16, C19); the leg
  IK and the step lift (C13); the support pins (C14);
- the sole's orientation drive (the pose drive turns the feet); the Ground
  Query for the toe's floor (it meets only level faces the feet skip, and
  Exarust's level has none); the footstep and scrape sounds of a foot's
  state change; the foot ground snap of the original's placement;
- the retail comparison (RR captures of a standing turn and a stop): the
  GPU's free memory (1.4 GB) allows only light arena runs; exanima01's
  stairs could not be loaded (out of memory) and are checked by a unit
  test instead.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: foot contacts, foot grip, the sole floor, the stance's
  level and the legs' extension (new); level friction, support, living
  bodies in motion, the body trace, getting up from the floor, steadying
  on the feet (modified).

## Impact

- `exa_physics` (`body.rs`, `body_balance.rs`, new `body_feet.rs` with
  `body_feet_test.rs`, `body_lift.rs`, `body_muscle.rs`, `body_inspect.rs`;
  the tests' standing fixture in `body_floor_test.rs`, `body_balance_test.rs`,
  `body_climb_test.rs`, `body_lift_test.rs`)
- `exa_game` (`limp.rs`, `stages/posture.rs`, `balance.rs`, `trace.rs`,
  `body_overlay.rs`)
- `docs/formats/RSQ.md`, `docs/PLAN.md`, `docs/PLAYER.md`,
  `docs/DEBUGGING.md`, the READMEs of `exa_physics` and `exa_game`
