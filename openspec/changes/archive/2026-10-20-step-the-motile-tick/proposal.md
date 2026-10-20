# Proposal

## Why

The `TMotileDynamics` tick kernel (20.3 KB) begins with a run of pure stages that were missing: posture scalars, the follow point and gain, the posture/support ramps, the tone weight, cadence, pitch impulse, angle wrap and the impact threshold. They are exact and can be shipped without the state machine.

## What Changes

- Posture: `lift = 0.9 + 0.09·lean − 0.6·(1−sag_b) − 0.1·(1−sag_a)` and `compliance = 1 − 0.01·lean + 0.05·(1−sag_a) + 0.30·(1−sag_b)`, both clamped with the original's NaN-second MINSS/MAXSS.
- Follow gain 0.0125, plus 0.0025 while active up to 0.15; the step adds the velocity gain (Y a tenth) and advances 0.6 of the delta.
- Posture ramp +0.004 to 1; support decay −0.02 to 0.
- Tone weight: warm states blend with 0.5; otherwise the flag and scale choose the error, its square or 0.97 of its square.
- Cadence `trunc((1 − 0.25·exertion)·1660/83)`.
- Pitch impulse `2·sign(v)·v²` with a 0.95 decay.
- Angle wrap into `(−π, π]`; impact `threshold·scale < |v|²`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the motile tick's pure stages.

## Impact

- `exa_physics::motile`, from `Posture`, `posture_scalars`, `follow_gain`, `follow_step`, `posture_ramp`, `support_decay`, `tone_weight`, `cadence_target`, `pitch_step`, `wrap_angle` and `impact_exceeds`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TMotileDynamics` becomes **27%** in the source rating. The headline rises slightly.
- Evidence: `0x10016924B` (posture), `0x100169493`/`0x100169516` (gain and step), `0x10016980F` (ramp/decay), `0x100169678` (tone weight), `0x100169CD4` (cadence), `0x100169DDB` (pitch), `0x10016C2A0` (wrap) and `0x10016A300` (impact). No new corpus files.
- Out of scope: the main dispatcher, the limb pull loop, the support/friction stage, helper A/B and the state machine.

## Phase

Phase 7, bodies. The tick's pure stages.
