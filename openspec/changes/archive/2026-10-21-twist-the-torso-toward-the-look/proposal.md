## Why

The original's body tick computes a torso twist before its sampler and
the sampler turns the spine by it, so a character's chest turns toward
where it looks (the cursor, a target) while its legs keep the heading.
Exarust's sampler step 7 was the identity, so the torso always faced the
heading.

Phase 3 (Bodies: skeleton, physics, control), milestone M3 "standing man";
the TMotileDynamics tick kernel lane (wave 2), the twist half of the
master plan's M8 (the head look is the other half).

## What Changes

- The twist (`exa_physics::body::locomotion::twist`, new `body_twist.rs`):
  the look in last tick's character frame, its yaw by the original's arc
  cosine approximation (`acos_approx`, `heading_of`), clamped to ±1 rad,
  followed at 0.02 / 0.05 rad a tick or the weapon's handling × 0.12 × the
  swing strength while attacking; the direction lowered by the crouch
  (0.3 of it attacking), × 0.75 (−y² casting with a low one); the look
  rotation along it; the identity while helpless or at a tone of 0.85 or
  less. Kept on `Locomotor` (`twist_yaw`, `twist`).
- The movement stage computes it each tick after locomotion, before the
  sampler.
- The sampler's step 7 (`twist_spine`): the midline chain from the root
  part (first child within 3 cm of the midline, recursively, the head
  excluded) takes the twist on its turns, the lumbar halfway back; the
  positions keep the pose.
- Docs: formats/RSQ.md (Torso twist, the pipeline table), PLAYER.md (the
  head looks).

Evidence:
- REA on `Exanima.exe` 0.9.5: the body tick's twist block and the living
  sampler's midline step in `context/re/exe.asm`; the midline chain's
  builder in the character init; the helpers (matrix product, lerp,
  Gram-Schmidt step, cross product, the arc cosine polynomial, the
  direction yaw, the look rotation with the vertical); constants read
  from the PE. Notes in `context/re/locomotion-kernel.md`.

Out of scope:
- the head look and its weight (the sampler's head step, M8);
- the run lean the direction's height starts from (an exertion value
  Exarust does not keep; 0);
- the twist in the captures (the original's starts capture it too);
- the processor's reciprocal square root estimate in the original's
  normalisations (Exarust normalises exactly).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `player-movement`: the torso twist toward the look.

## Impact

- `exa_physics`: new `body_twist.rs` under `body::locomotion`;
  `Locomotor::twist_yaw`, `Locomotor::twist`
- `exa_game`: `stages::movement` computes the twist; `stages::sampler`
  turns the spine (`spine`)
- `docs/formats/RSQ.md`, `docs/PLAYER.md`
