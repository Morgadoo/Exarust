## Why

In Exanima every living body is physical. Its muscles drive it toward the
animation, blows knock it about and daze it, and a dazed fighter sags until
it recovers. Exarust bodies were posed directly by their clips, so blows had
no physical effect.

## What Changes

- Living bodies get muscles. A pose drive pulls each particle of the
  tetrahedron parts toward the animated pose, with damping relative to the
  pose's own motion and gravity compensation by muscle tone. The gains follow
  the original's (0.084 per tick, 0.07 on hands, × tone² × energy⁴).
- Daze ("energy"): a blow takes up to 90 % of it, with a floor of 0.05; it
  recovers 0.005 per tick.
- Blade hits shove the struck part with a share of the blade's velocity and
  daze the body; knock-downs keep the blow's momentum as the body goes limp.
- Bodies are carried with their character controller's motion. Clips play in
  place: the root's horizontal travel over a clip is removed, which stops
  the walk cycle from sliding forward and snapping back.
- Player, opponent, placed characters and the character view are physical.
  The viewer gains `--shove <frame>`.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: living bodies driven by muscles; blows and daze.
- `combat`: hits shove and daze the body.
- `motion-playback`: clips play in place.

## Impact

- `exa_physics::body`, `exa_game::limp`, `exa_anim`, `exa_combat`,
  `exa_character`, the viewer, and `docs/formats/RSQ.md`.
