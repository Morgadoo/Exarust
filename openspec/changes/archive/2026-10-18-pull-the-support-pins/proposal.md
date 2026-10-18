# Proposal

## Why

A standing body is held on its stance spot by Exarust's own spring on the pelvis (0.168 × tone² × energy⁴ a tick). The original does not do that: with support, the two thorax pins and the pelvis pin pull horizontally toward targets on the character frame, clamped near the feet centre, and that is what keeps the chest over the feet. The frame anchor, the planted feet, the spawned pin factor and the collapse's pin-gain are already in place. This is the next named gap in Phase 3.

## What Changes

- In each solver iteration, a supported living body pulls its two thorax pins and its pelvis pin horizontally toward the character frame's targets. The targets stay within (60 − 60·crouch)·0.6·scale cm of the feet centre ((60 − 30·crouch)·0.6·scale while a foot is stepping; the pelvis pin within 60·scale cm). Crouch stays 0 until a crouch exists, so a standing thorax reach is 36·scale cm. One application of the pull, in centimetres through the body's cm conversion: after that clamp, raw = 0.2·e·|e|/60 when |e| < 60 cm and 12 cm along the error otherwise; subtract the point's horizontal displacement this tick; cap the result at 8 cm; multiply by S² · pin-gain · P · the spawned pin factor, and also by 0.3 while getting up. P is √E·T·E⁴. Pin-gain is the living value that starts at 0.08.
- With no support the pins apply no restoring pull, and their targets only shift 5·scale cm forward. While the body moves, the frame already follows the pelvis, so the pins do not nail it to one world spot.
- The capsule's horizontal stance hold no longer pulls the pelvis onto the stance spot. The capsule still carries the body across the ground and up the stair stopgap. Falls, get-ups, planted feet and balance are unchanged.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `physical-body`: the support pins replace the horizontal stance hold. A standing body is carried across the ground as before, and held over its feet by the pins.

## Impact

- `exa_physics`: the body iteration (`body.rs`) and a pin pull beside the feet and the frame. The stance-hold spring (`hold_on_frame`) stops pulling and braking the pelvis. Tests in the synthetic-body suite.
- Living docs: `docs/formats/RSQ.md` (Physical body), `docs/PLAN.md` (Phase 3 and §7), `crates/exa_physics/README.md`.

Evidence: `context/re/body-drive.md` §6 and `context/re/master-body.md` §5.3 (REA on the original). No retail addresses or decompiler names in code or specs. Tests use synthetic bodies.

Out of scope: recovery steps (C15), leg IK and the stair stopgap's removal (C13), gait-foot walking and removing the capsule (C18, C19), fitting part spheres to meshes, crouch, and any change to how falls, get-ups or planted feet choose their clips. Jumping has no flag yet, so the forward shift of pin targets while jumping is not added. Arena, story, combat systems beyond the existing body, AI, cloth and netplay stay deferred.
