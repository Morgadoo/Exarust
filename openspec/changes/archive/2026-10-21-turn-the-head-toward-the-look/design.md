## Context

The living sampler, for the head part (role 1), after the IK chain and
the twist: it clears its midline bit; when the condition has no helpless
bit and the motion index is under 0x2C, the weight rises 0.01 (double)
capped at 1; the look is the target's eye direction from the ground point
while a target and its timer last, else the smoothed cursor direction;
it is lerped toward the head's physical z axis by 1 − E² (single), taken
into the character frame (not yet rebuilt this tick), its height × 0.5,
normalised exactly; the head look routine then rewrites the head's
character-space rotation; the animated rotation takes it; the head's
anchor frame is relative to the physical thorax. Otherwise the anchor
frame is relative to the thorax's untwisted matrix and the weight is 0.

The head look routine: angles of the head's rotation (roll = asin(m01),
c = cos roll, |c| > 0.005 (double): pitch = atan2(−m21/c, m11/c), yaw =
atan2(−m02/c, m00/c); else pitch 0, yaw atan2(m20, m22); all in double);
with no helpless bit: yaw = heading(dir) by the arc-cosine polynomial,
|yaw| > 1.2566 (double) → ±1.2566, × 0.8 (double), followed by ±π/100
(double) from the kept yaw when farther; pitch = −asin~(dir.y) + an offset
(no writer: 0); else the kept yaw × 0.09 (double) and pitch 0; roll =
yaw × 0.1 (double); composed from (1 − w)·clip + w·look per angle with
single-precision sincos. Formats page: `docs/formats/RSQ.md` (Head look).

## Goals / Non-Goals

**Goals:** the head look as the original's, where it runs, with its anchor
frame.

**Non-Goals:** look targets; the clip ≥ 0x2C anchor frame; the offset.

## Decisions

- **Rows to columns.** The original's row-major matrices (row i = local
  axis i) are glam's columns; the formulas are written on the columns.
- **Rotation only.** As for the twist, the head's position keeps the pose.
- **The anchor frame** is flagged on the bearing by the sampler, which
  runs before the carry stage's muscle tuning in the same tick.

## Risks / Trade-offs

- The head now turns toward the cursor up to 57.6° and, dazed, keeps its
  physical facing. Not compared with a capture.

## Open Questions

- Who writes the pitch offset (no store found in `.text`): REA xrefs on
  the record copy that fills the character.
