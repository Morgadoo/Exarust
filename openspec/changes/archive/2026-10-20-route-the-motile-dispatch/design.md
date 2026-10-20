# Design

## Behaviour

The shock bit clears only after the unsigned clock passes the deadline. The dispatcher enters the posture block for balance below 0.6 or condition 0x300; the lying-still gate needs balance below 0.3 and motion below 25.

The attack arm base is `(1−clock²)·p·0.9+0.1`, or `p·0.9+0.1` for a body larger than 1.4 without the 0x30 weapon bits. The length adds `blend·(1−clock)·0.5666` when the clock is above 0.3, otherwise `blend·clock·1.3333` (the f32 clock at 0.3 is just above the double threshold). The other arm is `p` before 0.5 and `(1−clock)²·p·4` after; a stab clips the length by 0.3 and zeros the other.

The arm floor is `min(0.4·|turn|, 0.4)`, the parry ramp `0.8p+0.04`, gait `clamp(1−0.15g, 0.05, 0.2)`, the auto-parry deadline `ea4+400`, 0 or `d34+400`, and the switch phase 0x10004 above 0.6 progress with a log when bit 2 is clear.

## Model

`exa_physics::motile` adds the functions and `AttackArms`.

## Open questions

The animation calls, main draw, limb pull and remaining branches.

## Docs

`TMotileDynamics` is **30%**. The headline rises slightly.
