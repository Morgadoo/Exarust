## Why

Exarust's balance is its own invention: the centre of mass, projected
0.1 s ahead, against the line between the foot parts, scored
(2 − offset/threshold)⁸, with a counter that resets whenever the stance is
steady. Support S is a stand-in from C5 (1 while standing on a grounded
controller). The original measures balance at the chest against its foot
contacts, scores the lean's angle, keeps a counter that never drops under 80
once a body stands still, and asks the level for the ground under each foot
to decide support. S scales the original's muscles, lift and pins (C5, C7,
C8, C14), so it has to be the original's first.

Phase 3 (Bodies), milestone M3 "standing man". This is change C6 of the
body-controller plan; it depends on C1 and C2 (done) and replaces C5's
temporary support rule.

## What Changes

- The balance point is the chest, the mean of the two thorax pins, on the
  body's particles; its motion is smoothed twice a tick, fast (0.4) and
  slow (0.1), from the last two ticks.
- The support point is the closest point to the chest on the segment
  between the foot contacts (the parameter taken horizontally), moved
  halfway to their centre while standing. A landing leads the standing
  chest by the slow velocity × 5 (fading 0.05 a tick), a moving body with
  support by 3 × the fast velocity, and a crouch takes it 10 cm back.
- Score q = u_y⁸ standing, u_y⁴ moving, + 0.05 × the race's balance bonus,
  at most 1. A slow vertical chest speed over 3 cm/tick sets the counter to
  100 and, grounded, cuts q by (1 − 0.333(v − 3))², to 0 above 6.
- The counter grows by round-half-even(10 − 10q) while q ≤ 0.99; otherwise
  it falls by 5 to no less than 80 standing still, else by 10 to 0. At 100
  balance is q (and stays pinned there); below, balance recovers by
  (5 − v)·0.01 a tick at a slow chest speed under 5 cm/tick. Getting up,
  balance rises 0.05 a tick and the counter falls by 5 while above 2.
- Per leg, a Height Query (a 10 cm sphere from 70 cm above the contact,
  a 5 cm one from the contact while down or getting up, 5 cm along the
  lower leg's forward axis, swept 20 m down against the level; living
  bodies are not part of it). Ground higher than half the leg
  under the pelvis is out of reach (except getting up). A foot is grounded
  within 20 cm (30 moving) of its ground plus the sole height. Then
  has-support, the landing weight on regaining it, an airborne counter,
  balance 0 for a standing body without support, S = grounded × balance,
  and support at half balance for a body getting up within 30 cm of the
  ground under its pelvis.
- The foot contact a leg stands on is its foot part's origin, the ankle,
  as the original's muscle setup moves it (also for C5's leg relaxation,
  which becomes 0.2 for a straight standing leg instead of 0).
- S feeds the link muscles (leg limit 0.5·S²); arms stiffen to 0.5 below
  balance 0.6; the trace writes S, balance and the counter.
- Exarust's fall trigger and get-up flow stay (C11 replaces them): falls
  are decided in the fixed step from last tick's balance, Fall Forward when
  the lean is ahead of the body, else Fall Backward. A get-up's controller
  settles on the ground under the body.
- Exarust's own, until the body stands on its feet (C8, C18): a standing
  body whose controller stands on the ground keeps its balance when no foot
  is grounded (the capsule perches on rubble the feet hang beside).

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5: the balance routine, the
  chest point, the feet routine, the Height Query, the body tick's chest
  smoothing and muscle setup (`context/re/body-feet.md` §0–3, §7, §13 and
  the new §14, `body-drive.md` §5, `body-tick-order.md` §8,
  `body-controller-plan.md` C6). New this change: the contact is the ankle;
  leg length 85.15 cm; sole height from the foot's box; the counter is set
  to 100 by a fast vertical chest whether grounded or not; the Height
  Query's sphere starts with its lowest point at the probe; the plan's
  "q < 0.6 at 44 cm" leaves out the race bonus (52.2 cm at 130 cm).
- The installed `humanoid.rsq` and `humanbase.rfc` (pins, contacts, node
  translations) and `races.rdb` (balance bonus 1.0).
- Exarust runs through `gpu_run.sh` with a temporary probe: `play arena`
  standing and walking, `--shove 60`, `--opponent`, `play exanima01`,
  drops from 3 m.

Out of scope:
- recovery steps and footwork (C15), foot anchors, flags and friction by
  state (C12), the lift servo and target pelvis height (C8), the support
  pins (C14), the original's falls, staggers and get-ups (C11);
- the gait phase's probe sway and the gait-weighted fall of each foot's
  smoothed ground (C19), the combat condition's 5 cm probe;
- dead bodies and props answering the Height Query (bodies are not in the
  level's colliders yet).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: balance measured at the chest against the ankles, the
  Height Query, support from the feet, the fall direction from the lean,
  the arms off balance under 0.6, and the leg relaxation from the ankle.

## Impact

- `exa_physics` (`body.rs`: `PartSetup::pins`, the body's chest, legs and
  sole height, `Drive` keeps its `Balance`; new `body_balance.rs` with
  `Balance`, `Leg`, `Ragdoll::measure_balance` and the rules, tests in
  `body_balance_test.rs`; `body_muscle.rs`: `Bearing` gets `getting_up`,
  `crouch`, `carried` and loses `grounded`, `RaceMuscles::balance_bonus`;
  `contact.rs`: `Level::height_below`)
- `exa_game` (`balance.rs`: the fall trigger in the fixed step, the
  `Balance` component and its rule removed; `limp.rs`: the character tick
  measures balance, the bearing, the pins; `trace.rs`)
- `exarust` (`viewer.rs`: the race's balance bonus)
- `docs/formats/RSQ.md` (Physical body), `docs/PLAYER.md`, `docs/PLAN.md`,
  `crates/exa_physics/README.md`, `crates/exa_game/README.md`
