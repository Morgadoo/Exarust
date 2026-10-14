# Design

## Context

REA showed that `humanoid.rsq` is a bare chunk stream: a scene-node list
(bones; the two-digit name prefix is the bone index) and a motion set
`0x3DAB` (see `docs/formats/RSQ.md`).

## Decisions

- The bone list is parsed with the existing scene-node reader.
- Body parts of character models carry the same two-digit prefixes, so a part
  is driven by the bone with its index; the pelvis also takes the root
  position. Rotations are local `[x, y, z, w]` quaternions (confirmed visually:
  the A-pose arms come down to the sides in the walk).
- Time units are frames; playback assumes 60 per second until the original
  clock is measured.

## Risks / Trade-offs

- Seams between rigid body parts stay visible until the skin builder is
  reimplemented.
