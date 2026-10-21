## Context

The body tick, after its clock block and before its sampler: with the
condition free of the helpless bits and the tone over 0.85, the look
direction is taken into the character frame (rows dotted, last tick's
frame), its yaw computed by acos(z/|xz|) through a three-term polynomial
(signed by x, 0 under 1e-8 of horizontal length²), clamped to ±1, then a
kept yaw follows it by a step (0.02 moving, else (double) handling·0.12·P
in condition 2, else 0.05): beyond the step it moves by the step (down
when the kept yaw is at or above the target), within it it snaps. The
direction (sin, tilt, cos) of the kept yaw, the tilt less the crouch (0.3
of it, double, in condition 2; 1·crouch otherwise), normalised; casting
(condition 0x20) a height at or under 0 becomes −h², otherwise h·0.75;
normalised again (exact square root, y² + 0 + x² + z²). The twist is the
look matrix: z = the direction, y = the vertical less its part along z
(normalised), x = y × z (normalised). Otherwise the identity.

In the living sampler each part with the midline bit gets its animated
matrix = its IK-chain matrix × the twist (row vectors: the twist applied
after, in the character frame); a part whose parent is the root then
takes z = normalise(lerp(z twisted, z untwisted, 0.5)), y = normalise(y −
(y·z)z), x = normalise(y × z). The head clears its bit at its first
sample. The midline bit: from the root, the first child whose node lies
within 3 cm of the midline, and so on. Formats page: `docs/formats/RSQ.md`
(Torso twist, Sampler pipeline).

## Goals / Non-Goals

**Goals:** the twist and its application as the original's, wired where
it runs.

**Non-Goals:** the head look; the run lean; the twist in captures.

## Decisions

- **Rotation only.** The original twists the whole animated matrix, its
  translation too, but its support pins read the untwisted placement and
  its pose drive and link anchors read only turns; Exarust's targets carry
  both, so the twist turns only the spine's rotations and leaves their
  positions to the pose, which keeps the pins where the original's are.
- **The midline test** reads each part's rest translation across the
  body (|x| < 3 cm), the head (bone 10) leaving the chain as its first
  sample takes it out in the original.
- **The arc cosine** is the original's polynomial, since the twist's yaw
  rests on it (up to 4e-4 rad off the exact); the body turn keeps exact
  yaws, as Exarust's headings are yaws rather than the original's vectors.

## Risks / Trade-offs

- The torso now turns toward the cursor while standing (40 % of the way)
  and the look while walking; the arms keep their world pose as the chest
  turns under them, as in the original. Not compared with a capture.

## Open Questions

- The run lean (the exertion the locomotion and the feet write) and the
  head look's weight: with the head look change (M8).
