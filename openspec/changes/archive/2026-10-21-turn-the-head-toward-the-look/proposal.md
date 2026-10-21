## Why

The original's living sampler turns the head toward the look direction
(the smoothed cursor for the player) by a weight that ramps in, and
places the head's link anchors on the physical thorax so it keeps looking
whatever the chest does. Exarust's step 8 left the head on its clip.

Phase 3 (Bodies: skeleton, physics, control), milestone M3 "standing man";
the TMotileDynamics lane (wave 2), the head half of the master plan's M8
after the torso twist.

## What Changes

- `exa_physics::body::locomotion`: the original's arc sine polynomial
  (`asin_approx`), its angles of a turn and back (`euler_of`,
  `from_euler`), and the head look (`head_look`): the clip's angles moved
  by the weight toward the look's (its yaw by the arc cosine, within
  ±1.2566 rad, × 0.8, followed at most π/100 rad a tick; its pitch
  −asin(height); its roll a tenth of the yaw; held, the kept yaw × 0.09).
  `Locomotor` keeps the weight, the look yaw and the frame's yaw.
- The sampler's step 8: while not helpless and on a clip under action
  0x2C, the weight rises 0.01 a tick to 1 and the look (the smoothed
  cursor moved toward the head's physical forward by 1 − E², in last
  tick's frame, its height halved) turns the head; otherwise the weight
  is 0 and the head keeps its clip.
- The link muscles: a looking head's anchors relative to the thorax's
  physical transform (`Bearing::head_looks`).
- Docs: formats/RSQ.md (Head look, the pipeline table), PLAYER.md.

Evidence:
- REA on `Exanima.exe` 0.9.5: the sampler's head step and the head look
  routine in `context/re/exe.asm`, the angle helpers (decomposition with
  the double-precision arc sine, cosine and arc tangent; the composition
  with single-precision sines and cosines), the arc sine polynomial;
  constants read from the PE. Notes in `context/re/locomotion-kernel.md`.

Out of scope:
- the look at a target's eye point while a look timer runs (Exarust has
  no look target for the player);
- the head's relative frame for clips from 0x2C on (the twist is the only
  difference there);
- the pitch offset the original adds (no writer of it was found: 0).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `player-movement`: the head look.

## Impact

- `exa_physics`: `body_twist.rs` (the head look and its helpers),
  `Locomotor` (`frame_yaw`, `head_weight`, `head_yaw`), `Bearing::head_looks`,
  `body_muscle.rs` (the head's anchors)
- `exa_game`: `stages::sampler` (step 8, the look's input), `stages::movement`
  (the frame's yaw), `limp::bearing`
- `docs/formats/RSQ.md`, `docs/PLAYER.md`
