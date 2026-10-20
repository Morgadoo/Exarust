# Design

## Behaviour

The posture scalars are computed first in the tick. Lift begins at `0.9 + 0.09·lean`, loses `0.6·(1−sag_b)` and `0.1·(1−sag_a)`, and floors at 0. Compliance begins at `1 − 0.01·lean`, gains `0.05·(1−sag_a)` and `0.30·(1−sag_b)`, and caps at 1. Both clamps return the second operand on NaN, matching `MINSS`/`MAXSS`.

The follow point moves by 0.6 of `(target − point) + velocity·gain`, with the Y velocity scaled by a further 0.1. The gain is 0.0125, or the previous gain plus 0.0025 while active, capped at 0.15; the frozen flag forces 0.0125. Posture ramps by 0.004 to 1 while on and resets otherwise; support decays by 0.02 to 0.

The tone weight uses the warm-state blend `0.5e + 0.5`. Outside it, the 0x12F6 flag and the scale choose `e²·0.97` (scale above 1.3, state not 2), `e` (flagged), or `e²` (unflagged). Cadence truncates `(1 − 0.25·exertion)·1660/83`. Pitch applies `2·sign(v)·v²` when flagged and always decays by 0.95. The angle wraps with 2π; an impact crosses when `threshold·scale` is below the squared magnitude.

## Model

`exa_physics::motile` adds `Posture`, `posture_scalars`, `follow_gain`, `follow_step`, `posture_ramp`, `support_decay`, `tone_weight`, `cadence_target`, `pitch_step`, `wrap_angle` and `impact_exceeds`.

## Open questions

The main dispatcher, limb pull, support/friction, helper A/B and the state machine remain inside the kernel; the field names are inferred from use.

## Docs

`TMotileDynamics` is **27%**. The headline rises slightly.
