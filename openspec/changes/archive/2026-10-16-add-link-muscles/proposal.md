## Why

Exarust's living bodies follow their clips through a pose drive on every
part. The original turns its joints with link muscles instead: every slack
link pulls its point on the child toward where the clip's joint angle puts
it, given where the parent is. Only the pelvis, spine, head and feet keep a
pose drive. The gains change every tick: a standing human uses half its
tone (the race's stance factor), a moving one 0.7 to 1, the arms hang
loosely while walking and stiffen off balance, and a held pose is followed
softly.

Phase 3 (Bodies), milestone M3 "standing man". This is change C5 of the
body-controller plan. It depends on C1, C2 and C3 (done) and on Q2, which
is answered: a human's stance factor is the loaded `races.rdb` record's,
0.5. The arm stiffness states come from the answer to Q9.

## What Changes

- Every slack link of a living body becomes a link muscle. Its anchor is
  the part's point under the part's animated turn relative to its parent,
  at the part's origin, sampled every tick in the character tick and kept
  as weights on the parent. In each iteration, in cm: e = anchor − point
  (the anchor first drawn toward the point by the part's relaxation λ),
  plus the gap's change over the tick × D·k/(0.5|e|² + 1.4), with
  D = (E − 0.5)² (0 below E = 0.5), k = 50 moving, else 40 / stance
  factor, × 0.6 on the pelvis and legs; saturated e/(1 + 0.1|e|); times
  σ = strength × part factor × √E·T; the excess beyond the slack added at
  full weight; cut to 80 cm.
- Gains set by the character tick: stance factor s (the race's standing,
  rising 0.01 a tick toward 0.7 + 0.3·run while moving), T = tone·s;
  part factors 0.5, a foot half its lower leg's, the head a quarter of the
  thorax's, a thigh 0.25 standing and 0.2 moving, the arm parts 1.25 × the
  arm stiffness; the leg limit link's strength s ← 0.9s + 0.1t with
  t = 0.5·S² standing, 0.4 moving.
- Arm stiffness: 0.5 off balance, 0.2 dazed or striding, else easing to
  0.4·(0.5 + 0.5·race arm relaxation); floored by the turn left.
- Anchor relaxation λ for every part with link muscles, and a supported
  leg's from its reach.
- Support S until C6: 1 when the body has not fallen and its controller
  stands on the ground, else 0. The trace writes S, s and T.
- The pose drive is removed from thighs, shins, arms and hands (with the
  hands' own gain). Pelvis, lumbar, thorax, head and feet keep Exarust's
  drive until C7.
- The race's stance factor and arm relaxation reach the body through
  `Physical`; the game tells each body what its character does every tick.
- The duellist's strike range, Exarust's own, goes from 1.5 to 1.2 m:
  link-muscle arms lag their clip and reach 10–15 cm less far.

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5: the living link iteration,
  the PD history, the PD helpers, muscle setup, the living clip sampler,
  the leg stiffness routine, the arm stiffness routine and its states, the
  feet routine's leg relaxation, the stance factor update
  (`context/re/body-links.md` §4–5, `body-drive.md` §2–3,
  `body-tick-order.md` §5.4 and §8, `body-q8-q9.md` §7–9,
  `body-controller-plan.md` C5 and the answers to Q1–Q3). New this change:
  the sampler's relaxation rule applies to every part, not only the arms;
  a negative target leaves λ as it was; the shin's rest reach is twice its
  contact's distance; the standing leg mode gives a thigh 0.25.
- The installed `races.rdb` (C2): human stance factor 0.5, arm relaxation
  1.0.
- Exarust runs through `gpu_run.sh` with a temporary joint-error probe,
  against the build before this change: `character` Idle, Walk, Run and
  an attack; `play arena` standing and walking; `--shove 60`;
  `--opponent`; `play exanima01`; `--limp character`.

Out of scope:
- the original's attack, parry, recoil and power arm states, and the
  attack saturation offsets (attacks use the relaxed stiffness);
- balance and support (C6), the original's pose drive and fallen tone
  (C7), the lift servo (C8), clip blending (C10), weapon bodies and attack
  nudges (C21);
- the leg modes for sprint and backpedal and the stepping branch of the
  leg limit blend (no steps yet);
- body scale (other races' sizes) in the limb factors.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: link muscles, their gains, arm stiffness, anchor
  relaxation and support; the pose drive limited to the pelvis, spine,
  head and feet; the leg limit's strength follows the gait.

## Impact

- `exa_physics` (`body.rs`: `Part`, `Link`, `Drive`, `Physical`,
  `PartSetup::foot` becomes the contact point, `iterate`, `step`,
  `drive_parts`; new `body_muscle.rs` with `Bearing`, `RaceMuscles` and the
  muscle rules; tests in `body_muscle_test.rs`; the synthetic humanoid's
  joints get link muscles)
- `exa_game` (`limp.rs`: the bearing of each body, the race from
  `Physical`; `player.rs`: run blend and turn left; `trace.rs`)
- `exa_ai` (strike range), `exarust` (`viewer.rs`: race factors),
  `exa_formats` (doc of the arm relaxation)
- `docs/formats/RSQ.md` (Physical body), `docs/formats/OTHER.md` (Races),
  `docs/PLAYER.md`, `docs/PLAN.md`, `crates/exa_physics/README.md`
